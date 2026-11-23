use koma_source_sdk::json_utils::{extract_json_string, json_object_member, write_bytes};

#[derive(Copy, Clone)]
pub(super) struct Mirror {
    pub origin: &'static [u8],
    pub site: &'static [u8],
    pub app: &'static [u8],
}

// The user selected TW Webmota as the default. Saved explicit choices retain
// their original indices; absent or invalid settings use this same index.
pub(super) const DEFAULT_INDEX: usize = 4;
pub(super) const DEFAULT: Mirror = MIRRORS[DEFAULT_INDEX];
// WWW SEO canonical is an independent provider coordinate authority, not the
// current default entry. Changing the default must not invalidate old catalogs.
const WWW_CANONICAL_ORIGIN: &[u8] = b"https://www.baozimh.com";
pub(super) const MIRRORS: [Mirror; 17] = [
    Mirror { origin: b"https://cn.baozimh.com", site: b"https://cn.baozimh.com", app: b"https://appcn.baozimh.com" },
    Mirror { origin: b"https://tw.baozimh.com", site: b"https://tw.baozimh.com", app: b"https://appcn.baozimh.com" },
    Mirror { origin: b"https://www.baozimh.com", site: b"https://www.baozimh.com", app: b"https://appcn.baozimh.com" },
    Mirror { origin: b"https://cn.webmota.com", site: b"https://cn.webmota.com", app: b"https://appcn.baozimh.com" },
    Mirror { origin: b"https://tw.webmota.com", site: b"https://tw.webmota.com", app: b"https://appcn.baozimh.com" },
    Mirror { origin: b"https://www.webmota.com", site: b"https://www.webmota.com", app: b"https://appcn.baozimh.com" },
    Mirror { origin: b"https://cn.kukuc.co", site: b"https://cn.kukuc.co", app: b"https://appcn.baozimh.com" },
    Mirror { origin: b"https://tw.kukuc.co", site: b"https://tw.kukuc.co", app: b"https://appcn.baozimh.com" },
    Mirror { origin: b"https://www.kukuc.co", site: b"https://www.kukuc.co", app: b"https://appcn.baozimh.com" },
    Mirror { origin: b"https://cn.twmanga.com", site: b"https://cn.twmanga.com", app: b"https://appcn.baozimh.com" },
    Mirror { origin: b"https://tw.twmanga.com", site: b"https://tw.twmanga.com", app: b"https://appcn.baozimh.com" },
    Mirror { origin: b"https://www.twmanga.com", site: b"https://www.twmanga.com", app: b"https://appcn.baozimh.com" },
    Mirror { origin: b"https://cn.dinnerku.com", site: b"https://cn.dinnerku.com", app: b"https://appcn.baozimh.com" },
    Mirror { origin: b"https://tw.dinnerku.com", site: b"https://tw.dinnerku.com", app: b"https://appcn.baozimh.com" },
    Mirror { origin: b"https://www.dinnerku.com", site: b"https://www.dinnerku.com", app: b"https://appcn.baozimh.com" },
    Mirror { origin: b"https://appcn.baozimh.com", site: b"https://appcn.baozimh.com/baozimhapp", app: b"https://appcn.baozimh.com" },
    Mirror { origin: b"https://appgb.baozimh.com", site: b"https://appgb.baozimh.com/baozimhapp", app: b"https://appgb.baozimh.com" },
];

pub(super) fn selected(request: &[u8]) -> Mirror {
    let value = json_object_member(request, b"settings")
        .and_then(|settings| extract_json_string(settings, b"preferred_mirror"));
    value.and_then(|value| core::str::from_utf8(value).ok())
        .and_then(|value| value.parse::<usize>().ok())
        .and_then(|index| MIRRORS.get(index).copied()).unwrap_or(DEFAULT)
}

pub(super) fn join(output: &mut [u8], base: &[u8], path: &[u8]) -> Option<usize> {
    let mut cursor = 0;
    (write_bytes(output, &mut cursor, base) && write_bytes(output, &mut cursor, path))
        .then_some(cursor)
}

// Only the extension's declared sites may supply a relative work/chapter path.
// An absolute foreign URL never becomes an arbitrary network endpoint.
pub(super) fn relative_path(url: &[u8]) -> Option<&[u8]> {
    let path = if url.starts_with(b"/") && !url.starts_with(b"//") {
        url
    } else {
        MIRRORS.iter().find_map(|mirror| url.strip_prefix(mirror.origin)
            .filter(|rest| rest.starts_with(b"/")))?
    };
    Some(path.strip_prefix(b"/baozimhapp").unwrap_or(path))
}

pub(super) fn canonical_path<'a>(request: &[u8], url: &'a [u8]) -> Option<&'a [u8]> {
    let chosen = selected(request);
    // App and regional pages may use the official WWW SEO canonical. This
    // validates provider coordinates only; it never authorizes or follows it.
    let path = url.strip_prefix(chosen.origin)
        .or_else(|| url.strip_prefix(WWW_CANONICAL_ORIGIN))?;
    if !path.starts_with(b"/") { return None; }
    Some(path.strip_prefix(b"/baozimhapp").unwrap_or(path))
}

// App pages currently emit a syntactically valid but unusable SEO subdomain.
// Read only its coordinate path: it must remain in Baozi's own HTTPS namespace
// and name the caller's exact slug plus an explicit alphanumeric alias suffix.
// This is not an HTTP target or a cookie/session authorization.
pub(super) fn app_canonical_alias_path<'a>(request: &[u8], url: &'a [u8], slug: &[u8]) -> Option<&'a [u8]> {
    let selected = selected(request);
    if selected.site == selected.origin { return None; }
    let rest = url.strip_prefix(b"https://")?;
    let slash = rest.iter().position(|b| *b == b'/')?;
    let host = &rest[..slash];
    if !host.ends_with(b".baozimh.com") || !host.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'-' || *b == b'.') {
        return None;
    }
    let path = rest[slash..].strip_prefix(b"/baozimhapp").unwrap_or(&rest[slash..]);
    let owner = path.strip_prefix(b"/comic/")?;
    let owner = owner.strip_suffix(b"/").unwrap_or(owner);
    let suffix = owner.strip_prefix(slug)?.strip_prefix(b"_")?;
    if suffix.is_empty() || !suffix.iter().all(u8::is_ascii_alphanumeric) { return None; }
    Some(path)
}
