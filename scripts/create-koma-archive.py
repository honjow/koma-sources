#!/usr/bin/env python3
"""Write a reproducible source package or delivery ZIP for unchanged bytes.

Keep build.sh's explicit manifest/WASM/icon order. Fix container metadata only;
changed compiler output or compression implementations can still change hashes.
"""
import argparse
from pathlib import Path
import stat
import zipfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path)
    parser.add_argument("--delivery", action="store_true")
    parser.add_argument("members", nargs="+")
    args = parser.parse_args()
    if len(set(args.members)) != len(args.members):
        parser.error("archive members must be unique")
    if args.delivery:
        attribution = {"LICENSE", "LICENSE-APACHE-2.0", "NOTICE", "NOTICE.md"}
        if (len(args.members) < 2 or not args.members[0].endswith(".koma")
                or not set(args.members[1:]).issubset(attribution)):
            parser.error("expected one source package and its attribution files")
    elif len(args.members) != 3 or args.members[:2] != ["manifest.json", "source.wasm"]:
        parser.error("expected manifest.json, source.wasm and the validated root icon")
    members = [Path(name) for name in args.members]
    for name, path in zip(args.members, members):
        if path.name != name or "\\" in name or path.is_symlink() or not path.is_file():
            parser.error("members must be regular root files")
        if path.resolve() == args.archive.resolve():
            parser.error("archive must not replace an input member")
    with zipfile.ZipFile(args.archive, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for name, path in zip(args.members, members):
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.create_system = 3
            info.create_version = 20
            info.extract_version = 20
            info.external_attr = (stat.S_IFREG | 0o644) << 16
            info.internal_attr = 0
            info.extra = b""
            info.comment = b""
            archive.writestr(info, path.read_bytes(), compress_type=zipfile.ZIP_DEFLATED, compresslevel=9)


if __name__ == "__main__":
    main()
