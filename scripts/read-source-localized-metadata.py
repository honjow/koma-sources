#!/usr/bin/env python3
"""Project explicit display names and optional navigation homepage metadata.

Preserve existing localizedNames spelling and values. homepage is independent of
webSessionOrigins and never grants cookie/session permissions. Unrelated fields
are ignored rather than merged into generated manifests or indexes.
"""
from pathlib import Path
import json
import re
import sys
from urllib.parse import urlsplit

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


def navigation_homepage(value):
    if (not isinstance(value, str) or not value or not valid_unicode(value)
            or any(char.isspace() for char in value) or CONTROL.search(value)
            or any(char in value for char in '\\<>"')):
        raise ValueError("invalid homepage: expected safe HTTP(S) navigation URL")
    try:
        parsed = urlsplit(value)
        if (parsed.scheme.lower() not in ("http", "https") or not parsed.hostname
                or parsed.username is not None or parsed.password is not None
                or "%" in parsed.hostname):
            raise ValueError("invalid homepage URL")
        # Accessing port validates numeric/range syntax without making a request.
        parsed.port
    except ValueError as error:
        raise ValueError("invalid homepage URL") from error
    return value


def extract(metadata):
    if not isinstance(metadata, dict):
        return {}
    result = {}
    if "homepage" in metadata:
        result["homepage"] = navigation_homepage(metadata["homepage"])
    if "localizedNames" not in metadata:
        return result
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
    result["localizedNames"] = names
    return result


def main():
    path = Path(sys.argv[1])
    metadata = json.loads(path.read_text()) if path.exists() else {}
    print(json.dumps(extract(metadata), ensure_ascii=False, separators=(",", ":")))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError) as error:
        raise SystemExit(str(error))
