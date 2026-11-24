#!/usr/bin/env python3
"""Project author metadata into the installed source manifest.

Navigation, access and content identity have distinct owners. The archive
projection retains the legacy wire names accepted by released clients.
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


class MetadataObject(dict):
    """Keep raw member counts for optional catalogs before canonical deduplication."""
    def __init__(self, pairs):
        super().__init__(pairs)
        self.pairs = pairs


def raw_entries(value):
    return value.pairs if isinstance(value, MetadataObject) else list(value.items())


def locale_tag(value):
    if (not isinstance(value, str) or len(value) > 63
            or not TAG.fullmatch(value) or CONTROL.search(value)):
        return None
    return value.lower()


def localized_texts(value, limit=None):
    # Optional translations are individually disposable; the original strict
    # localizedNames contract below is intentionally not weakened.
    if not isinstance(value, dict) or len(raw_entries(value)) > 16:
        return {}
    seen = set()
    duplicated = set()
    texts = {}
    for tag, text in raw_entries(value):
        normalized = locale_tag(tag)
        if normalized is None:
            continue
        if normalized in seen:
            duplicated.add(normalized)
            texts = {key: value for key, value in texts.items() if key.lower() != normalized}
            continue
        seen.add(normalized)
        if (not isinstance(text, str) or not text.strip(TRIM) or not valid_unicode(text)
                or (limit is not None and (text != text.strip(TRIM) or CONTROL.search(text) or utf16_length(text) > limit))
                or (limit is None and re.search(r"[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]", text))):
            continue
        texts[tag] = text
    return {} if duplicated else texts


def optional_member(value, key):
    if not isinstance(value, dict) or sum(name == key for name, _ in raw_entries(value)) != 1:
        return None
    return value.get(key)


def project_localization(metadata, result):
    # These are existing display roles only. Messages never create an auth
    # descriptor, new controls, permissions, or operation targets.
    if sum(name == "localization" for name, _ in raw_entries(metadata)) > 1:
        raise ValueError("duplicate localization catalog")
    catalog = optional_member(metadata, "localization")
    presentation = {"version": 1, "texts": {}}
    if (isinstance(catalog, dict) and type(optional_member(catalog, "version")) is int
            and optional_member(catalog, "version") == 1 and isinstance(optional_member(catalog, "messages"), dict)):
        default = optional_member(catalog, "defaultLocale")
        if locale_tag(default) is not None:
            presentation["defaultLocale"] = default
        messages = catalog["messages"]
        roles = {"source.name": ("name", 128), "source.description": ("description", None),
                 "source.author": ("author", 128)}
        if result.get("authentication", {}).get("kind") == "credentials":
            for role in ("authentication.usernameLabel", "authentication.passwordLabel"):
                roles[role] = (role, 128)
        for key, (role, limit) in roles.items():
            # Repeated JSON message keys are not selected by order.
            if sum(name == key for name, _ in raw_entries(messages)) != 1:
                continue
            texts = localized_texts(messages.get(key), limit)
            if texts:
                presentation["texts"][role] = texts
    names = result.get("localizedNames")
    projected_names = presentation["texts"].get("name")
    if names is not None and projected_names is not None:
        merged = dict(names)
        old = {tag.lower(): (tag, text) for tag, text in names.items()}
        for tag, text in projected_names.items():
            if tag.lower() in old:
                if old[tag.lower()][1] != text:
                    raise ValueError("localization source.name conflicts with localizedNames")
            else:
                merged[tag] = text
        if len(merged) > 16:
            raise ValueError("localizedNames projection exceeds 16 entries")
        names = merged
    elif projected_names is not None:
        names = projected_names
    if names is not None:
        result["localizedNames"] = dict(names)
        # A description-only Catalog must not give legacy names a new default
        # language. Keep their original resolver unless source.name opts in.
        if names and projected_names is not None:
            presentation["texts"]["name"] = dict(names)
    if presentation["texts"]:
        result["presentation"] = presentation


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
    if not isinstance(value, dict) or set(value) - {"kind", "loginUrl", "usernameLabel", "passwordLabel", "credentialStorage", "userAgent", "loginUrls", "sessionCookieNames", "webToken"}:
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
    if "webToken" in value:
        token = value["webToken"]
        if (kind != "web" or "#" in login_url or not isinstance(token, dict)
                or set(token) - {"version", "storageKey", "apiOrigin", "verifyPath", "verifyIdentityPath", "imageKeyDerivation"}
                or type(token.get("version")) is not int or token["version"] != 1
                or not isinstance(token.get("storageKey"), str)
                or not re.fullmatch(r"[A-Za-z0-9_.-]{1,64}", token["storageKey"])
                or not isinstance(token.get("verifyPath"), str) or len(token["verifyPath"]) > 256
                or not re.fullmatch(r"/[A-Za-z0-9_-]+(?:/[A-Za-z0-9_-]+)*", token["verifyPath"])
                or not isinstance(token.get("verifyIdentityPath"), list)
                or not 1 <= len(token["verifyIdentityPath"]) <= 4
                or any(not isinstance(key, str) or key in {"__proto__", "prototype", "constructor"}
                       or not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_-]{0,63}", key)
                       for key in token["verifyIdentityPath"])
                or ("imageKeyDerivation" in token and token["imageKeyDerivation"] != "sha512_cbc")):
            raise ValueError("invalid web token descriptor")
        website = re.fullmatch(r"https://www\.([a-z0-9-]+\.[a-z]{2,})", origin)
        destination = token.get("apiOrigin")
        if destination != origin and (website is None or destination != "https://api." + website[1]):
            raise ValueError("web token API must use the same origin or the corresponding api hostname")
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


def request_limits(rules):
    if not isinstance(rules, list) or len(rules) > 8:
        raise ValueError("invalid requestLimits")
    groups = set()
    def integer(value, high):
        return type(value) is int and 1 <= value <= high
    def text(value, pattern):
        return isinstance(value, str) and re.fullmatch(pattern, value) is not None
    def strings(values, pattern):
        return (isinstance(values, list) and 1 <= len(values) <= 16 and
                all(text(value, pattern) for value in values))
    for rule in rules:
        if not isinstance(rule, dict):
            raise ValueError("invalid request limit")
        group = rule.get("group")
        if (not text(group, r"[a-z][a-z0-9_-]{0,31}") or group in groups
                or not integer(rule.get("permits"), 2147483647)
                or not integer(rule.get("periodMs"), 2147483647000)):
            raise ValueError("invalid request limit window")
        groups.add(group)
        if "hosts" in rule and ("firstLabels" in rule or
                not strings(rule["hosts"], r"[a-z0-9][a-z0-9.-]{0,252}")):
            raise ValueError("invalid request limit hosts")
        if "firstLabels" in rule:
            labels = rule["firstLabels"]
            if (not isinstance(labels, list) or not 1 <= len(labels) <= 8 or
                    any(not isinstance(label, dict) or
                        not text(label.get("prefix"), r"[a-z0-9][a-z0-9-]{0,30}") or
                        type(label.get("decimalSuffix", False)) is not bool for label in labels)):
                raise ValueError("invalid request limit labels")
        if "requestHeaders" in rule and not strings(rule["requestHeaders"], r"[a-z0-9][a-z0-9_-]{0,63}"):
            raise ValueError("invalid request limit header names")
        if any(type(rule.get(flag, False)) is not bool for flag in ("perOrigin", "oncePerRequest")):
            raise ValueError("invalid request limit scope")
        if "setting" in rule:
            setting = rule["setting"]
            if (not isinstance(setting, dict) or not text(setting.get("key"), r"[A-Za-z][A-Za-z0-9_-]{0,127}") or
                    not integer(setting.get("maxPermits", 2147483647), 2147483647) or
                    any(type(setting.get(flag, False)) is not bool for flag in ("windowValue", "keepUntilRestart"))):
                raise ValueError("invalid request limit setting")
    return rules


def project_website_policy(metadata):
    policy = metadata.get("websitePolicy", {})
    if not isinstance(policy, dict):
        raise ValueError("invalid websitePolicy")
    if "identity" not in metadata:
        return metadata.get("websitePolicy")

    identity = metadata["identity"]
    if not isinstance(identity, dict) or not identity or set(identity) - {"chapter", "mihon"}:
        raise ValueError("invalid identity fields")
    if "chapter" not in identity:
        return metadata.get("websitePolicy")
    if "chapterIdentity" in policy:
        raise ValueError("declare the chapter rule only in identity.chapter")

    # Match the installed host's existing chapterIdentity contract for the
    # author-facing rule. Old author metadata retains its original projection.
    chapter = identity["chapter"]
    required = {"workPrefix", "chapterPrefix", "chapterSeparator"}
    suffixes = {"workSuffix", "chapterSuffix"}
    known = required | suffixes | {"titleSeparator", "encodedLegacy"}
    if not isinstance(chapter, dict) or not required <= chapter.keys() <= known:
        raise ValueError("invalid identity.chapter fields")

    def token(value):
        return isinstance(value, str) and re.fullmatch(r"[A-Za-z0-9/_:.-]{1,32}", value)

    if not all(token(chapter[key]) for key in (required | suffixes) & chapter.keys()):
        raise ValueError("invalid identity.chapter token")
    if "encodedLegacy" in chapter:
        legacy = chapter["encodedLegacy"]
        if ("titleSeparator" in chapter or not isinstance(legacy, dict)
                or set(legacy) != {"prefix", "separator", "hexSuffixLength"}
                or not token(legacy["prefix"]) or not token(legacy["separator"])
                or type(legacy["hexSuffixLength"]) is not int
                or not 1 <= legacy["hexSuffixLength"] <= 64):
            raise ValueError("invalid identity.chapter.encodedLegacy")
    elif chapter.get("titleSeparator") not in ("#", "|", "~"):
        raise ValueError("identity.chapter requires a titleSeparator or encodedLegacy")

    # v1 clients consume this key for details, reader and library refresh.
    return {**policy, "chapterIdentity": chapter}


def mihon_import_identity(rule):
    if (not isinstance(rule, dict) or not {"sourceIds", "work"} <= rule.keys() <=
            {"sourceIds", "work", "chapter", "samePageOrder"}):
        raise ValueError("invalid identity.mihon fields")
    ids = rule["sourceIds"]
    if (not isinstance(ids, list) or not ids or not all(
            isinstance(value, str) and re.fullmatch(r"-?\d{1,19}", value) for value in ids)):
        raise ValueError("invalid identity.mihon.sourceIds")
    for key in ("work", "chapter"):
        if key not in rule:
            continue
        path = rule[key]
        text_fields = {"pathPrefix", "pathSuffix", "idPrefix", "idSuffix"}
        allowed = text_fields | ({"pathIdSegment", "targetIdSegment", "pathWorkIdSeparator"} if key == "chapter" else {"pathTitleAfterId"})
        if (not isinstance(path, dict) or not {"pathPrefix", "idPrefix"} <= path.keys() <= allowed or not all(
                    isinstance(path[field], str) and re.fullmatch(r"[A-Za-z0-9._:/?=&,-]{0,128}", path[field])
                    for field in path.keys() & text_fields) or (path["pathPrefix"] != "" and not path["pathPrefix"].startswith("/") and not re.match(r"^https://[A-Za-z0-9.-]+/", path["pathPrefix"]))):
            raise ValueError("invalid identity.mihon." + key)
        if ("pathTitleAfterId" in path and (type(path["pathTitleAfterId"]) is not bool or
                path["pathTitleAfterId"] and
                (not path["pathPrefix"].endswith("/") or path.get("pathSuffix") != "/"))):
            raise ValueError("invalid identity.mihon.work.pathTitleAfterId")
        if "pathWorkIdSeparator" in path and (
                not path["pathPrefix"] or "pathIdSegment" in path or
                not isinstance(path["pathWorkIdSeparator"], str) or
                not re.fullmatch(r"[A-Za-z0-9._:/?=&,-]{1,128}", path["pathWorkIdSeparator"])):
            raise ValueError("invalid identity.mihon.chapter.pathWorkIdSeparator")
        for field, minimum in (("pathIdSegment", -8), ("targetIdSegment", 0)):
            if field in path and (type(path[field]) is not int or not minimum <= path[field] <= 7):
                raise ValueError("invalid identity.mihon.chapter." + field)
    if ("samePageOrder" in rule and type(rule["samePageOrder"]) is not bool or
            rule.get("samePageOrder") and "chapter" not in rule):
        raise ValueError("invalid identity.mihon.samePageOrder")
    return rule


def extract(metadata, web_origins=None):
    if not isinstance(metadata, dict):
        return {}
    result = {}
    policy = project_website_policy(metadata)
    if policy is not None:
        result["websitePolicy"] = policy
    if "mihon" in metadata.get("identity", {}):
        result["mihonImport"] = mihon_import_identity(metadata["identity"]["mihon"])
    if "requestLimits" in metadata:
        result["requestLimits"] = request_limits(metadata["requestLimits"])
    if "comments" in metadata:
        comments = metadata["comments"]
        if (not isinstance(comments, dict) or set(comments) != {"version", "scopes"}
                or type(comments["version"]) is not int or comments["version"] != 1
                or not isinstance(comments["scopes"], list) or not comments["scopes"]
                or any(scope not in ("manga", "chapter") for scope in comments["scopes"])
                or len(set(comments["scopes"])) != len(comments["scopes"])):
            raise ValueError("invalid comments capability")
        result["comments"] = comments
    if "requiredImageDecodes" in metadata:
        kinds = metadata["requiredImageDecodes"]
        if (not isinstance(kinds, list)
                or any(kind not in ("jm_stripes_v1", "wasm_image_v1") for kind in kinds)
                or len(set(kinds)) > 1):
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
        project_localization(metadata, result)
        return result
    names = metadata["localizedNames"]
    if not isinstance(names, dict) or len(raw_entries(names)) > 16:
        raise ValueError("invalid localizedNames: expected object with at most 16 entries")
    seen = set()
    for tag, name in raw_entries(names):
        if (len(tag) > 63 or CONTROL.search(tag) or not TAG.fullmatch(tag)
                or tag.lower() in seen or not isinstance(name, str)):
            raise ValueError("invalid localizedNames language tag or value")
        if (name != name.strip(TRIM) or not name or utf16_length(name) > 128
                or not valid_unicode(name) or CONTROL.search(name)):
            raise ValueError("invalid localizedNames display name")
        seen.add(tag.lower())
    result["localizedNames"] = names
    project_localization(metadata, result)
    return result


def main():
    path = Path(sys.argv[1])
    metadata = json.loads(path.read_text(), object_pairs_hook=MetadataObject) if path.exists() else {}
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
