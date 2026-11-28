#!/usr/bin/env python3
"""Resume an immutable, SHA-named GitHub source library release from CI artifacts."""

import argparse
import hashlib
import json
import os
import re
import time
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.parse import quote, urlencode, urlsplit
from urllib.request import Request, urlopen

# Pace release mutations below GitHub's usual 500 content-generating requests/hour.
WRITE_INTERVAL = 8
MAX_THROTTLE_RETRIES = 5
MAX_THROTTLE_WAIT = 900
NOTES = (
    "Automatic source library. index.json lists installable packages; "
    "catalogue.json records every production source and its availability status. "
    "Build success does not certify all provider content."
)


class GitHub:
    def __init__(self, base_url, repo, token):
        self.base = base_url.rstrip("/") + "/repos/" + repo
        self.token = token
        self.last_write = float("-inf")

    def request(self, method, path, *, payload=None, upload=None, allow_404=False):
        url = self.base + path if path.startswith("/") else path
        headers = {
            "Accept": "application/vnd.github+json",
            "Authorization": "Bearer " + self.token,
            "X-GitHub-Api-Version": "2022-11-28",
        }
        body = None
        if payload is not None:
            body = json.dumps(payload).encode()
            headers["Content-Type"] = "application/json"
        elif upload is not None:
            body = upload.read_bytes()
            headers["Content-Type"] = "application/octet-stream"

        for attempt in range(MAX_THROTTLE_RETRIES + 1):
            if method != "GET":
                time.sleep(max(0, self.last_write + WRITE_INTERVAL - time.monotonic()))
                self.last_write = time.monotonic()
            try:
                with urlopen(Request(url, data=body, headers=headers, method=method), timeout=120) as response:
                    return json.load(response) if response.status != 204 else None
            except HTTPError as exc:
                if allow_404 and exc.code == 404:
                    return None
                detail = exc.read().decode(errors="replace")
                retry_after = exc.headers.get("Retry-After")
                remaining = exc.headers.get("X-RateLimit-Remaining")
                limited = exc.code == 429 or (
                    exc.code == 403 and (
                        bool(retry_after) or remaining == "0" or "secondary rate limit" in detail.lower()
                    )
                )
                transient = exc.code in (500, 502, 503, 504)
                if not (limited or transient) or attempt == MAX_THROTTLE_RETRIES:
                    raise RuntimeError(f"{method} {url}: HTTP {exc.code}: {detail}") from None
                if transient and not limited:
                    wait = min(15 * (attempt + 1), 60)
                    print(f"GitHub transient error (HTTP {exc.code}); waiting {wait:.0f}s before retrying.", flush=True)
                    time.sleep(wait)
                    continue
                delays = [1 if retry_after else 60 * 2**attempt]
                if retry_after:
                    delays.append(int(retry_after))
                if remaining == "0" and exc.headers.get("X-RateLimit-Reset"):
                    delays.append(int(exc.headers["X-RateLimit-Reset"]) - time.time() + 1)
                wait = max(delays)
                if wait > MAX_THROTTLE_WAIT:
                    raise RuntimeError(f"GitHub requested a {wait:.0f}s wait; rerun only the publish job later.")
                print(f"GitHub rate limit (HTTP {exc.code}); waiting {wait:.0f}s before retrying.", flush=True)
                time.sleep(wait)
            except (URLError, TimeoutError) as exc:
                if attempt == MAX_THROTTLE_RETRIES:
                    raise RuntimeError(f"{method} {url}: {exc}") from None
                wait = min(15 * (attempt + 1), 60)
                print(f"GitHub connection error ({exc}); waiting {wait:.0f}s before retrying.", flush=True)
                time.sleep(wait)

    def paginate(self, path, per_page=100):
        page = 1
        while True:
            items = self.request("GET", f"{path}?per_page={per_page}&page={page}")
            yield from items
            if len(items) < per_page:
                break
            page += 1


def expected_assets(dist):
    files = [dist / name for name in ("index.json", "repo.json", "catalogue.json")]
    for folder in ("release-packages", "release-icons"):
        members = sorted((dist / folder).iterdir())
        if not members:
            raise ValueError(f"Empty release artifact directory: {folder}")
        files.extend(members)
    expected = {}
    for file in files:
        if file.name in expected:
            raise ValueError(f"Duplicate release asset name: {file.name}")
        with file.open("rb") as stream:
            digest = "sha256:" + hashlib.file_digest(stream, "sha256").hexdigest()
        expected[file.name] = (file, file.stat().st_size, digest)
    return expected


def matches(asset, spec):
    return (
        asset is not None
        and asset["state"] == "uploaded"
        and asset["size"] == spec[1]
        and asset.get("digest") == spec[2]
    )


def publish(api, sha, branch, expected):
    tag = "sources-" + sha
    branch_path = "/git/ref/heads/" + quote(branch, safe="/")

    def still_current():
        return api.request("GET", branch_path)["object"]["sha"] == sha

    if not still_current():
        print(f"Skipping superseded revision {sha}.")
        return
    # Direct tag lookup avoids slow/504 full /releases scans when already published;
    # fall back to small-page /releases scan (per_page=10) to resume unpublished drafts.
    release = api.request("GET", f"/releases/tags/{quote(tag, safe='')}", allow_404=True)
    if not release:
        release = next((item for item in api.paginate("/releases", per_page=10) if item["tag_name"] == tag), None)
    if release and not release["draft"]:
        print("This revision is already published; keeping its assets unchanged.")
        return
    if not release:
        release = api.request("POST", "/releases", payload={
            "tag_name": tag, "target_commitish": sha, "draft": True,
            "name": f"Source library {sha[:8]}", "body": NOTES,
        })
    if release["target_commitish"] != sha:
        raise RuntimeError("Existing draft does not target the expected commit.")
    release_path = f"/releases/{release['id']}"
    upload_url = release["upload_url"].split("{", 1)[0]
    if urlsplit(upload_url).netloc != "uploads.github.com" or urlsplit(upload_url).scheme != "https":
        raise ValueError("Unexpected GitHub release upload host.")
    assets = {item["name"]: item for item in api.paginate(release_path + "/assets")}
    extra = sorted(assets.keys() - expected.keys())
    if extra:
        raise RuntimeError(f"Unexpected assets in draft; leaving them untouched: {', '.join(extra)}")

    reused = 0
    for name, spec in expected.items():
        asset = assets.get(name)
        if matches(asset, spec):
            reused += 1
            continue
        if asset is not None:
            api.request("DELETE", f"/releases/assets/{asset['id']}")
        uploaded = api.request("POST", upload_url + "?" + urlencode({"name": name}), upload=spec[0])
        if not matches(uploaded, spec):
            raise RuntimeError(f"GitHub returned unexpected size, state, or digest for {name}.")
        print(f"Uploaded {name}", flush=True)

    final = {item["name"]: item for item in api.paginate(release_path + "/assets")}
    if final.keys() != expected.keys() or not all(matches(final[name], spec) for name, spec in expected.items()):
        raise RuntimeError("Draft assets do not match the build artifact; not publishing.")
    if not still_current():
        print(f"Leaving superseded draft {tag} unpublished.")
        return
    api.request("PATCH", release_path, payload={"draft": False, "make_latest": "true"})
    print(f"Published {tag}: {len(expected)} verified assets, {reused} reused.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dist", type=Path, required=True)
    parser.add_argument("--branch", required=True)
    args = parser.parse_args()
    sha = os.environ["GITHUB_SHA"]
    if not re.fullmatch("[a-f0-9]{40}", sha):
        raise ValueError("Invalid source commit SHA.")
    api = GitHub(os.environ["GITHUB_API_URL"], os.environ["GITHUB_REPOSITORY"], os.environ["GH_TOKEN"])
    publish(api, sha, args.branch, expected_assets(args.dist))


if __name__ == "__main__":
    main()
