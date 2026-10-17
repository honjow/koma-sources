#!/usr/bin/env python3
"""Validate the required source identity resource without changing its bytes."""
import argparse
import json
import stat
import sys
import warnings
from pathlib import Path

MAX_BYTES = 1024 * 1024
MAX_PIXELS = 1024 * 1024
FORMATS = {".png": "PNG", ".jpg": "JPEG", ".jpeg": "JPEG", ".gif": "GIF", ".webp": "WEBP", ".ico": "ICO"}
ICON_NAMES = tuple("icon" + ext for ext in FORMATS)

class ValidationError(Exception):
    pass

def validate_icon(path):
    path = Path(path)
    try:
        entry = path.lstat()
        if not stat.S_ISREG(entry.st_mode):
            raise ValidationError("icon must be a regular local file, not a symlink")
        if not 0 < entry.st_size <= MAX_BYTES:
            raise ValidationError("icon must contain 1..1048576 bytes")
        expected = FORMATS.get(path.suffix.lower())
        if expected is None:
            raise ValidationError("unsupported icon extension")
        from PIL import Image
        with warnings.catch_warnings():
            warnings.simplefilter("error", Image.DecompressionBombWarning)
            with Image.open(path) as image:
                if image.format != expected:
                    raise ValidationError("icon bytes do not match its extension")
                width, height = image.size
                if width <= 0 or height <= 0 or width * height > MAX_PIXELS:
                    raise ValidationError("icon must contain 1..1048576 decoded pixels")
                image.verify()
            with Image.open(path) as image:
                image.load()
    except ValidationError:
        raise
    except ImportError as error:
        raise ValidationError("Python Pillow is required for actual image decoding") from error
    except Exception as error:
        raise ValidationError("icon cannot be read/decoded: {}".format(error)) from error
    return path

def source_icon(directory, package=False):
    directory = Path(directory)
    icons = [directory / name for name in ICON_NAMES if (directory / name).exists() or (directory / name).is_symlink()]
    if len(icons) != 1:
        raise ValidationError("source requires exactly one root icon.png/jpg/jpeg/gif/webp/ico; found {}".format(len(icons)))
    icon = validate_icon(icons[0])
    if package:
        try:
            declared = json.loads((directory / "manifest.json").read_text())["icon"]
        except (OSError, ValueError, KeyError) as error:
            raise ValidationError("production manifest must declare icon") from error
        if declared != icon.name:
            raise ValidationError("manifest icon does not match the root asset")
    return icon

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--icon", help="validate one local root or published index asset")
    group.add_argument("--source-dir", help="require exactly one source-owned icon")
    group.add_argument("--package-dir", help="also require manifest.icon to match")
    parser.add_argument("--print-name", action="store_true")
    args = parser.parse_args()
    try:
        icon = validate_icon(args.icon) if args.icon else source_icon(args.package_dir or args.source_dir, bool(args.package_dir))
    except ValidationError as error:
        print("source icon validation failed: {}".format(error), file=sys.stderr)
        return 1
    if args.print_name:
        print(icon.name)
    else:
        print("source icon validation passed: {}".format(icon))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
