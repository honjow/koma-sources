#!/usr/bin/env python3
"""Incrementally update the flat GitHub Release directory used by installed Koma apps."""

import argparse
from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time
from urllib.error import HTTPError, URLError
from urllib.parse import quote
from urllib.request import Request, urlopen

TAGS = ("source-library-a", "source-library-b")
DOCUMENTS = ("index.json", "repo.json", "catalogue.json")
ASSET_NAME = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,239}\Z")
DIGEST = re.compile(r"sha256:[a-f0-9]{64}\Z")
ASSET_LIMIT = 1000
NOTES = (
    "Koma source library. Use the repository's latest release URL for subscriptions. "
    "This publishing slot is reused after the other slot becomes latest."
)


@dataclass(frozen=True)
class File:
    path: Path
    size: int
    digest: str

    @classmethod
    def read(cls, path):
        with path.open("rb") as stream:
            digest = "sha256:" + hashlib.file_digest(stream, "sha256").hexdigest()
        return cls(path, path.stat().st_size, digest)


def matches(asset, file):
    return (asset is not None and asset["state"] == "uploaded"
            and asset["size"] == file.size and asset.get("digest") == file.digest)


def asset_name(value):
    if not isinstance(value, str) or not ASSET_NAME.fullmatch(value):
        raise ValueError(f"Invalid flat release asset name: {value!r}")
    return value

class UploadRetry(RuntimeError):
    def __init__(self, message, delay):
        super().__init__(message)
        self.delay = delay



class GitHub:
    def __init__(self, base_url, repo, token):
        self.base = base_url.rstrip("/") + "/repos/" + repo
        self.repo = repo
        self.token = token

    def request(self, method, path, payload=None, *, allow_404=False):
        headers = {
            "Accept": "application/vnd.github+json",
            "Authorization": "Bearer " + self.token,
            "X-GitHub-Api-Version": "2022-11-28",
        }
        data = None
        if payload is not None:
            headers["Content-Type"] = "application/json"
            data = json.dumps(payload).encode()
        request = Request(self.base + path, data=data, headers=headers, method=method)
        for attempt in range(5):
            try:
                with urlopen(request, timeout=60) as response:
                    return None if response.status == 204 else json.load(response)
            except HTTPError as exc:
                if exc.code == 404 and allow_404:
                    return None
                detail = exc.read().decode(errors="replace")
                after = exc.headers.get("Retry-After")
                remaining = exc.headers.get("X-RateLimit-Remaining")
                limited = exc.code == 429 or (
                    exc.code == 403 and (after or remaining == "0" or "secondary rate limit" in detail.lower())
                )
                # Creating a draft is not idempotent. A subsequent run finds an
                # existing draft if GitHub actually created it before disconnecting.
                if attempt == 4 or method == "POST" or not (limited or exc.code in (500, 502, 503, 504)):
                    raise RuntimeError(f"{method} {path}: HTTP {exc.code}: {detail}") from None
                wait = int(after) if after and after.isdigit() else (60 * 2**attempt if limited else 2**attempt)
                reset = exc.headers.get("X-RateLimit-Reset")
                if remaining == "0" and reset and reset.isdigit():
                    wait = max(wait, int(reset) - time.time() + 1)
                if wait > 900:
                    raise RuntimeError(f"GitHub asked to wait {wait:.0f}s. Retry the publish job later.")
                print(f"GitHub HTTP {exc.code}; retrying in {wait:.0f}s.", flush=True)
                time.sleep(wait)
            except (URLError, TimeoutError):
                if attempt == 4 or method == "POST":
                    raise
                time.sleep(2**attempt)

    def paginate(self, path, per_page=100):
        for page in range(1, 1000):
            rows = self.request("GET", f"{path}?per_page={per_page}&page={page}")
            yield from rows
            if len(rows) < per_page:
                return
        raise RuntimeError(f"Unexpectedly large GitHub listing: {path}")

    def latest(self):
        return self.request("GET", "/releases/latest", allow_404=True)

    def slot(self, tag, sha):
        release = self.request("GET", f"/releases/tags/{tag}", allow_404=True)
        if release is None:
            release = next((row for row in self.paginate("/releases", per_page=10)
                            if row["tag_name"] == tag), None)
        if release is None:
            release = self.request("POST", "/releases", {
                "tag_name": tag, "target_commitish": sha, "draft": True,
                "name": tag, "body": NOTES,
            })
        if release["tag_name"] != tag or release.get("immutable"):
            raise RuntimeError(f"Release slot is not writable: {tag}")
        return release

    def assets(self, release):
        return {asset["name"]: asset for asset in self.paginate(f"/releases/{release['id']}/assets")}

    def upload(self, release, paths):
        if not paths:
            return
        # Use the CLI's bounded uploader. Ordinary incremental updates make
        # one call and never wait. A cold slot can hit GitHub's write limits.
        result = subprocess.run(
            ["gh", "release", "upload", release["tag_name"], *map(str, paths), "--repo", self.repo],
            text=True, capture_output=True,
        )
        if result.returncode == 0:
            return
        message = " ".join((result.stderr or result.stdout).split())[:500]
        if re.search(r"secondary rate limit|API rate limit exceeded|HTTP 429", message, re.I):
            raise UploadRetry(message, 180)
        if re.search(r"HTTP 50[0234]|unexpected EOF|connection reset|timed? out", message, re.I):
            raise UploadRetry(message, 15)
        raise RuntimeError(f"GitHub upload exited with {result.returncode}: {message}")

    def remove(self, asset):
        self.request("DELETE", f"/releases/assets/{asset['id']}", allow_404=True)

    def download(self, release, asset, directory):
        name = asset_name(asset["name"])
        target = directory / name
        # Public release downloads do not need the Actions token, including
        # redirects to the GitHub CDN.
        url = f"https://github.com/{self.repo}/releases/download/{quote(release['tag_name'], safe='')}/{name}"
        with urlopen(url, timeout=60) as response, target.open("wb") as out:
            shutil.copyfileobj(response, out)
        file = File.read(target)
        if not matches(asset, file):
            raise RuntimeError(f"Could not verify previous release asset: {name}")
        return file


def source_assets(dist):
    expected = {}
    for folder in ("release-packages", "release-icons"):
        for path in sorted((dist / folder).iterdir()):
            name = asset_name(path.name)
            if name in expected:
                raise ValueError(f"Duplicate build asset: {name}")
            expected[name] = File.read(path)
    return expected


def records(path):
    rows = json.loads(path.read_bytes())
    if not isinstance(rows, list) or not rows:
        raise ValueError(f"Expected a nonempty source list: {path.name}")
    return rows


def previous_paths(api, release, assets, directory):
    if release is None:
        return {}, {}
    index = api.download(release, assets["index.json"], directory)
    required = {}
    by_source = {}
    for entry in records(index.path):
        for field in ("pkg", "icon"):
            name = asset_name(entry[field])
            asset = assets[name]
            if asset["state"] != "uploaded" or not DIGEST.fullmatch(asset.get("digest") or ""):
                raise RuntimeError(f"Unverifiable asset in the current index: {name}")
            if field == "pkg" and asset["digest"] != "sha256:" + entry["sha256"]:
                raise RuntimeError(f"Current index checksum mismatch: {name}")
            required[name] = asset
            by_source[(entry["id"], field)] = name
    return required, by_source


def choose_names(files, available, existing, previous, index):
    # Retain the old name when it already denotes these bytes. If a source is
    # rebuilt with different bytes at the same version, keep both reachable.
    previous_names = {}
    for entry in index:
        for field in ("pkg", "icon"):
            name = asset_name(entry[field])
            prior = previous.get((entry["id"], field))
            if prior is not None and matches(available[prior], files[name]):
                previous_names[name] = prior
    names = {}
    for name, file in files.items():
        target = previous_names.get(name, name)
        if any(asset and asset["state"] == "uploaded" and not matches(asset, file)
               for asset in (available.get(target), existing.get(target))):
            stem, ext = target.rsplit(".", 1)
            target = asset_name(f"{stem}-{file.digest[7:31]}.{ext}")
        if target in names.values():
            raise ValueError(f"Conflicting planned asset: {target}")
        names[name] = target
    return names


def write_documents(dist, directory, index, files, names):
    for entry in index:
        if files[asset_name(entry["pkg"])].digest != "sha256:" + entry["sha256"]:
            raise ValueError(f"Output index checksum differs from package: {entry['id']}")
        asset_name(entry["icon"])
    docs = {}
    for name, rows in (("index.json", index), ("catalogue.json", records(dist / "catalogue.json"))):
        for record in rows:
            for field in ("pkg", "icon"):
                record[field] = names[asset_name(record[field])]
        path = directory / name
        path.write_text(json.dumps(rows, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        docs[name] = File.read(path)
    docs["repo.json"] = File.read(dist / "repo.json")
    return docs


def upload_missing(api, release, existing, wanted, staging):
    reused = sum(matches(existing.get(name), file) for name, file in wanted.items())
    for attempt in range(4):
        inputs = []
        for name, file in wanted.items():
            asset = existing.get(name)
            if matches(asset, file):
                continue
            if asset:
                api.remove(asset)
            path = staging / name
            if file.path != path:
                shutil.copyfile(file.path, path)
            inputs.append(path)
        try:
            api.upload(release, inputs)
            break
        except UploadRetry as error:
            if attempt == 3:
                raise
            wait = error.delay * 2**attempt
            print(f"{error} Waiting {wait}s before continuing the same slot.", flush=True)
            time.sleep(wait)
            # A failed batch can still have written hundreds of valid files.
            # Query their digests before retrying, and leave the active slot alone.
            existing = api.assets(release)
    print(f"{len(wanted) - reused} uploaded, {reused} reused.", flush=True)


def verify(assets, wanted):
    missing = [name for name, file in wanted.items() if not matches(assets.get(name), file)]
    if missing:
        raise RuntimeError(f"Missing or mismatched release assets: {', '.join(missing[:12])}")


def publish(api, sha, branch, dist):
    branch_path = "/git/ref/heads/" + quote(branch, safe="/")
    title = f"Koma source library ({sha[:8]})"

    def current():
        return api.request("GET", branch_path)["object"]["sha"] == sha

    if not current():
        print(f"Skipping superseded revision {sha}.")
        return
    active = api.latest()
    if active and active["tag_name"] in TAGS and active["body"] == f"{NOTES}\n\nRevision: {sha}":
        print(f"Revision {sha} already published.")
        return
    tag = TAGS[1] if active and active["tag_name"] == TAGS[0] else TAGS[0]
    files = source_assets(dist)
    active_assets = api.assets(active) if active else {}

    with tempfile.TemporaryDirectory(prefix="koma-source-release-") as temp:
        root = Path(temp)
        previous_dir = root / "previous"
        previous_dir.mkdir()
        files_dir = root / "files"
        files_dir.mkdir()
        docs_dir = root / "documents"
        docs_dir.mkdir()
        previous, by_source = previous_paths(api, active, active_assets, previous_dir)
        if active and active["tag_name"] in TAGS:
            index = records(dist / "index.json")
            names = choose_names(files, previous, active_assets, by_source, index)
            docs = write_documents(dist, docs_dir, index, files, names)
            if (all(matches(active_assets.get(names[name]), file) for name, file in files.items())
                    and all(matches(active_assets.get(name), file) for name, file in docs.items())):
                print(f"No changes to the {len(files)} source assets or the public index. Nothing to upload.")
                return

        release = api.slot(tag, sha)
        existing = api.assets(release)
        index = records(dist / "index.json")
        names = choose_names(files, previous, existing, by_source, index)
        docs = write_documents(dist, docs_dir, index, files, names)
        wanted = {names[name]: value for name, value in files.items()}
        # Keep links from the current index working through the latest switch,
        # including cached index readers and checksum retries.
        local_by_digest = {(file.digest, file.size): file for file in files.values()}
        retained = {}
        for name, asset in previous.items():
            if name in wanted:
                if not matches(asset, wanted[name]):
                    raise RuntimeError(f"New publication would overwrite a current asset: {name}")
            elif matches(existing.get(name), File(Path(name), asset["size"], asset["digest"])):
                retained[name] = asset
            else:
                same = local_by_digest.get((asset["digest"], asset["size"]))
                wanted[name] = same if same else api.download(active, asset, previous_dir)
        all_required = set(wanted) | retained.keys() | docs.keys()
        if len(all_required) > ASSET_LIMIT:
            raise RuntimeError(f"Current and next indexes need {len(all_required)} assets; limit is {ASSET_LIMIT}.")
        old_optional = [name for name in existing if name not in all_required]
        old_optional.sort(key=lambda name: existing[name]["created_at"])
        excess = max(0, len(all_required) + len(old_optional) - ASSET_LIMIT)
        if not current() or (api.latest() or {}).get("id") != (active or {}).get("id"):
            print("Publication baseline changed before staging.")
            return
        for name in old_optional[:excess]:
            api.remove(existing[name])
        upload_missing(api, release, existing, wanted, files_dir)
        before_index = api.assets(release)
        verify(before_index, wanted)
        for name, asset in retained.items():
            if not matches(before_index.get(name), File(Path(name), asset["size"], asset["digest"])):
                raise RuntimeError(f"Missing or mismatched previous asset: {name}")
        upload_missing(api, release, before_index, docs, docs_dir)
        verify(api.assets(release), docs)

        if not current() or (api.latest() or {}).get("id") != (active or {}).get("id"):
            print(f"Leaving {tag} unpromoted because the publication baseline changed.")
            return
        api.request("PATCH", f"/releases/{release['id']}", {
            "name": title, "body": f"{NOTES}\n\nRevision: {sha}",
            "draft": False, "make_latest": "true",
        })
        if (api.latest() or {}).get("id") != release["id"]:
            raise RuntimeError("Verified release was not selected as latest.")
        print(f"Published {sha} as {tag}; {len(all_required)} verified files.", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dist", required=True, type=Path)
    parser.add_argument("--branch", required=True)
    args = parser.parse_args()
    sha = os.environ["GITHUB_SHA"]
    if not re.fullmatch(r"[a-f0-9]{40}", sha):
        raise ValueError("Invalid source commit SHA.")
    api = GitHub(os.environ["GITHUB_API_URL"], os.environ["GITHUB_REPOSITORY"], os.environ["GH_TOKEN"])
    publish(api, sha, args.branch, args.dist)


if __name__ == "__main__":
    main()
