#!/usr/bin/env python3
"""Retain complete, explicitly pinned official packages before release publication."""
import argparse
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import re
import stat
import sys
import tempfile
from urllib.request import Request, urlopen
import zipfile

sys.dont_write_bytecode = True

class PinError(ValueError):
    pass

def sha(data):
    return hashlib.sha256(data).hexdigest()

def entry_sha(entry):
    return sha(json.dumps(entry, ensure_ascii=False, sort_keys=True,
                          separators=(",", ":")).encode())

def verified(data, expected, label):
    if sha(data) != expected:
        raise PinError(label + " SHA256 mismatch")
    return data

def records(data, label):
    rows = json.loads(data)
    if not isinstance(rows, list) or any(not isinstance(row, dict) or
            not isinstance(row.get("id"), str) or not row["id"] for row in rows):
        raise PinError(label + " must be a source array")
    if len({row["id"] for row in rows}) != len(rows):
        raise PinError(label + " contains duplicate source IDs")
    return rows

def download(url):
    with urlopen(Request(url, headers={"User-Agent": "Koma-release-pins/1"}), timeout=30) as response:
        data = response.read(32 * 1024 * 1024 + 1)
    if len(data) > 32 * 1024 * 1024:
        raise PinError("release asset exceeds download budget")
    return data

def icons():
    spec = importlib.util.spec_from_file_location("pin_icon", Path(__file__).with_name("validate-source-icon.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

def check_package(package, icon, entry, pin):
    with zipfile.ZipFile(io.BytesIO(package)) as archive:
        manifest = json.loads(archive.read("manifest.json"))
        icon_name = manifest.get("icon")
        members = archive.infolist()
        if (icon_name not in icons().ICON_NAMES or len(members) != 3 or
                {item.filename for item in members} != {"manifest.json", "source.wasm", icon_name}):
            raise PinError("pinned archive requires exactly manifest/WASM/root icon")
        if any(item.is_dir() or stat.S_IFMT(item.external_attr >> 16) not in (0, stat.S_IFREG)
               or item.flag_bits & 1 for item in members) or archive.testzip() is not None:
            raise PinError("pinned archive has invalid members or CRC")
        if manifest.get("id") != pin["id"] or manifest.get("version") != pin["version"]:
            raise PinError("pinned manifest identity/version mismatch")
        for key in ("id", "name", "version", "lang", "nsfw", "author", "description",
                    "contentRating", "minAppVersion", "homepage", "localizedNames"):
            if key in entry and manifest.get(key) != entry[key]:
                raise PinError("pinned manifest/index mismatch: " + key)
        wasm = archive.read("source.wasm")
        if not wasm.startswith(b"\0asm\x01\0\0\0"):
            raise PinError("pinned source.wasm is not a WASM v1 module")
        if archive.read(icon_name) != icon:
            raise PinError("published icon differs from package icon")
        return {"manifestSha256": sha(archive.read("manifest.json")), "wasmSha256": sha(wasm)}

def validate_config(config):
    if (not isinstance(config, dict) or type(config.get("version")) is not int or
            config["version"] != 1 or not isinstance(config.get("pins"), list) or
            not re.fullmatch(r"https://github\.com/[A-Za-z0-9_-]+/[A-Za-z0-9_.-]+", config.get("repository", ""))):
        raise PinError("invalid explicit release pin configuration")
    seen = set()
    for pin in config["pins"]:
        if not isinstance(pin, dict) or not isinstance(pin.get("id"), str) or not pin["id"] or pin["id"] in seen:
            raise PinError("invalid or duplicate pinned source ID")
        seen.add(pin["id"])
        if not re.fullmatch(r"sources-[0-9a-f]{40}", pin.get("tag", "")):
            raise PinError("pins require a fixed sources-<commit> tag, never latest")
        for key in ("indexSha256", "packageSha256", "iconSha256"):
            if not isinstance(pin.get(key), str) or not re.fullmatch(r"[0-9a-f]{64}", pin[key]):
                raise PinError("missing explicit SHA256: " + key)
        if (not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", pin.get("version", "")) or
                not isinstance(pin.get("reason"), str) or not pin["reason"].strip()):
            raise PinError("pins require a version and reason")
        if (not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.-]*\.koma", pin.get("pkg", "")) or
                not re.fullmatch(r"[A-Za-z0-9_-]+-icon\.(png|jpg|jpeg|gif|webp|ico)", pin.get("icon", ""))):
            raise PinError("pinned assets must use flat release filenames")
    return config["pins"]

def atomic_write(path, data):
    name = None
    try:
        with tempfile.NamedTemporaryFile(dir=path.parent, prefix=".release-pin-", delete=False) as file:
            name = file.name
            file.write(data)
        os.replace(name, path)
    finally:
        if name is not None and os.path.exists(name):
            os.unlink(name)

def apply_pins(output, config, fetch=download):
    pins = validate_config(config)
    if not pins:
        return {"pins": [], "networkAssets": 0}
    output = Path(output)
    index = output / "index.json"
    catalogue_path = output / "catalogue.json"
    before = records(index.read_bytes(), "current index")
    catalogue_bytes = catalogue_path.read_bytes()
    catalogue = {row["id"]: row for row in records(catalogue_bytes, "candidate catalogue")}
    ids = {row["id"] for row in before}
    pinned_ids = {pin["id"] for pin in pins}
    available = {key for key, row in catalogue.items() if row.get("available") is True}
    if available - ids or ids - available - pinned_ids:
        raise PinError("pins require this release's complete index/catalogue")
    for pin in pins:
        candidate = catalogue.get(pin["id"])
        if (candidate is None or candidate.get("available") is not False or
                not isinstance(candidate.get("reason"), str) or not candidate["reason"].strip()):
            raise PinError("pinned candidate must remain explicitly unavailable: " + pin["id"])
    cache = {}
    def asset(url, expected, label):
        if url not in cache:
            cache[url] = fetch(url)
        return verified(cache[url], expected, label)
    replacements, assets, receipt = {}, {}, []
    with tempfile.TemporaryDirectory(prefix="koma-release-pins-") as temporary:
        for pin in pins:
            base = config["repository"] + "/releases/download/" + pin["tag"] + "/"
            old = records(asset(base + "index.json", pin["indexSha256"], "official index"), "official index")
            selected = [row for row in old if row["id"] == pin["id"]]
            if len(selected) != 1:
                raise PinError("pinned source absent from verified official index")
            entry = selected[0]
            if (entry.get("version") != pin["version"] or entry.get("pkg") != pin["pkg"] or
                    entry.get("icon") != pin["icon"] or entry.get("sha256") != pin["packageSha256"] or
                    entry.get("available") is False):
                raise PinError("pin does not match the complete official index entry")
            package = asset(base + pin["pkg"], pin["packageSha256"], "official package")
            icon = asset(base + pin["icon"], pin["iconSha256"], "official icon")
            details = check_package(package, icon, entry, pin)
            path = Path(temporary) / pin["icon"]
            path.write_bytes(icon)
            icons().validate_icon(path)  # Reuse production decode and image budgets.
            replacements[pin["id"]] = entry
            for directory, name, data in (("release-packages", pin["pkg"], package), ("release-icons", pin["icon"], icon)):
                target = output / directory / name
                if ((target in assets and assets[target] != data) or target.is_symlink() or
                        (target.exists() and target.read_bytes() != data)):
                    raise PinError("release staging filename has different bytes: " + name)
                assets[target] = data
            receipt.append({"id": pin["id"], "tag": pin["tag"], "version": entry["version"],
                            "minAppVersion": entry.get("minAppVersion"), "pkg": pin["pkg"],
                            "indexSha256": pin["indexSha256"], "packageSha256": sha(package),
                            "icon": pin["icon"], "iconSha256": sha(icon),
                            "entrySha256": entry_sha(entry), **details,
                            "candidateVersion": catalogue[pin["id"]].get("version"),
                            "candidateAvailable": False, "reason": catalogue[pin["id"]]["reason"]})
    after = [replacements.get(row["id"], row) for row in before]
    after.extend(replacements[pin["id"]] for pin in pins if pin["id"] not in ids)
    if catalogue_path.read_bytes() != catalogue_bytes:
        raise PinError("candidate catalogue changed during validation")
    # Finish every validation before touching staging. Write assets before the
    # atomic index replacement. This tool never calls the Release write API.
    for target, data in assets.items():
        target.parent.mkdir(parents=True, exist_ok=True)
        if not target.exists():
            atomic_write(target, data)
    atomic_write(index, (json.dumps(after, ensure_ascii=False, indent=2) + "\n").encode())
    return {"pins": receipt, "indexCountBefore": len(before), "indexCount": len(after),
            "otherEntriesUnchanged": [row for row in before if row["id"] not in pinned_ids] ==
                                     [row for row in after if row["id"] not in pinned_ids],
            "candidateCatalogueUnchanged": True, "networkAssets": len(cache),
            "entrySha256Encoding": "UTF-8 JSON with sorted keys and compact separators; all fields"}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pins", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    try:
        print(json.dumps(apply_pins(args.output_dir, json.loads(args.pins.read_text())), ensure_ascii=False, indent=2))
    except (OSError, ValueError, KeyError, zipfile.BadZipFile) as error:
        raise SystemExit("release pin validation failed: " + str(error))

if __name__ == "__main__":
    main()
