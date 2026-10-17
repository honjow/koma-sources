#!/usr/bin/env python3
"""Extract only localizedNames under the existing App display-name contract.

Preserve tag spelling and values in the package; App normalizes on consumption.
Unrelated metadata is ignored and never merged into the generated manifest.
"""
from pathlib import Path
import json
import re
import sys

TAG = re.compile(r"[A-Za-z]{2,8}(?:-[A-Za-z0-9]{1,8}){0,5}", re.ASCII)
CONTROL = re.compile(r"[\x00-\x1f\x7f]")
# ECMAScript String.trim whitespace (not Python's broader str.strip set).
TRIM = "\u0009\u000a\u000b\u000c\u000d\u0020\u00a0\u1680\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a\u2028\u2029\u202f\u205f\u3000\ufeff"


def utf16_length(value):
    return len(value.encode("utf-16-le", errors="surrogatepass")) // 2


def valid_unicode(value):
    try:
        value.encode("utf-16-le", errors="surrogatepass").decode("utf-16-le")
        return True
    except UnicodeError:
        return False


def extract(metadata):
    if not isinstance(metadata, dict) or "localizedNames" not in metadata:
        return {}
    names = metadata["localizedNames"]
    if not isinstance(names, dict) or len(names) > 16:
        raise ValueError("invalid localizedNames: expected object with at most 16 entries")
    seen = set()
    for tag, name in names.items():
        if (len(tag) > 63 or CONTROL.search(tag) or not TAG.fullmatch(tag)
                or tag.lower() in seen or not isinstance(name, str)):
            raise ValueError("invalid localizedNames language tag or value")
        if (name != name.strip(TRIM) or not name or utf16_length(name) > 128
                or not valid_unicode(name) or CONTROL.search(name)):
            raise ValueError("invalid localizedNames display name")
        seen.add(tag.lower())
    return {"localizedNames": names}


def main():
    path = Path(sys.argv[1])
    metadata = json.loads(path.read_text()) if path.exists() else {}
    print(json.dumps(extract(metadata), ensure_ascii=False, separators=(",", ":")))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError) as error:
        raise SystemExit(str(error))
