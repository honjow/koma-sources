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


def authentication_descriptor(value, web_origins):
    if not isinstance(value, dict) or set(value) - {"kind", "loginUrl", "usernameLabel", "passwordLabel", "credentialStorage", "userAgent", "loginUrls", "sessionCookieNames"}:
        raise ValueError("invalid authentication descriptor")
    kind = value.get("kind")
    login_url = value.get("loginUrl")
    if kind not in ("web", "credentials") or not isinstance(login_url, str) or utf16_length(login_url) > 4096:
        raise ValueError("invalid authentication descriptor")
    navigation_homepage(login_url)
    parsed = urlsplit(login_url)
    if parsed.scheme != "https":
        raise ValueError("authentication requires HTTPS")
    host = parsed.hostname.encode("idna").decode("ascii").lower()
    if ":" in host:
        host = "[" + host + "]"
    origin = "https://" + host + ((":" + str(parsed.port)) if parsed.port not in (None, 443) else "")
    if kind == "web" and origin not in web_origins:
        raise ValueError("web authentication requires its declared exact web-session origin")
    if kind == "credentials" and value.get("credentialStorage") not in ("private", "memory"):
        raise ValueError("credentials must declare private or memory credentialStorage")
    if kind == "web" and "credentialStorage" in value:
        raise ValueError("web authentication cannot declare native credential storage")
    if "userAgent" in value:
        ua = value["userAgent"]
        if kind != "web" or not isinstance(ua, str) or not ua.strip(TRIM) or utf16_length(ua) > 1024 or any(ord(c) < 32 or ord(c) == 127 for c in ua):
            raise ValueError("invalid authentication user agent")
    if "loginUrls" in value or "sessionCookieNames" in value:
        urls = value.get("loginUrls")
        names = value.get("sessionCookieNames")
        if (kind != "credentials" or not isinstance(urls, list) or not 1 <= len(urls) <= 8
                or any(not isinstance(url, str) or utf16_length(url) > 4096 for url in urls)
                or login_url not in urls or len(set(urls)) != len(urls)):
            raise ValueError("invalid native session login URLs")
        origins = set()
        for url in urls:
            navigation_homepage(url)
            parsed_url = urlsplit(url)
            if parsed_url.scheme != "https" or "#" in url:
                raise ValueError("native session login requires exact HTTPS URLs")
            origin = (parsed_url.hostname.encode("idna").decode("ascii").lower(), parsed_url.port or 443)
            if origin in origins:
                raise ValueError("duplicate native session login origin")
            origins.add(origin)
        if (not isinstance(names, list) or not 1 <= len(names) <= 16
                or any(not isinstance(name, str) or not re.fullmatch(r"[A-Za-z0-9_.-]{1,64}", name)
                       for name in names)
                or len(set(names)) != len(names)):
            raise ValueError("invalid native session cookie names")
    for field in ("usernameLabel", "passwordLabel"):
        if field in value and (not isinstance(value[field], str) or not value[field].strip(TRIM)
                               or utf16_length(value[field]) > 128 or not valid_unicode(value[field])):
            raise ValueError("invalid authentication field label")
    return dict(value)


def extract(metadata, web_origins=None):
    if not isinstance(metadata, dict):
        return {}
    result = {}
    if "requiredImageDecodes" in metadata:
        kinds = metadata["requiredImageDecodes"]
        if not isinstance(kinds, list) or any(kind != "jm_stripes_v1" for kind in kinds):
            raise ValueError("invalid requiredImageDecodes: unsupported decoder kind")
        result["requiredImageDecodes"] = sorted(set(kinds))
    if "authentication" in metadata:
        result["authentication"] = authentication_descriptor(metadata["authentication"], web_origins if web_origins is not None else metadata.get("webSessionOrigins", []))
    # Chronology belongs to the unchanged source output, not the App's display
    # sort. Missing means unverified; never infer it from names or chapter IDs.
    if "chapterOrder" in metadata:
        if metadata["chapterOrder"] not in ("newest_first", "oldest_first"):
            raise ValueError("invalid chapterOrder: expected explicit source chronology")
        result["chapterOrder"] = metadata["chapterOrder"]
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
    origin_file = path.parent / "web-session-origins.json"
    web_origins = json.loads(origin_file.read_text()) if origin_file.exists() else metadata.get("webSessionOrigins", [])
    projected = extract(metadata, web_origins)
    # Production sources must not publish a dead ordinary WebView action.
    # This is navigation metadata only; it grants no HTTP/session permission.
    if "--require-homepage" in sys.argv[2:] and "homepage" not in projected:
        raise ValueError("production source requires an explicit official homepage")
    print(json.dumps(projected, ensure_ascii=False, separators=(",", ":")))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError) as error:
        raise SystemExit(str(error))
