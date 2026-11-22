use koma_source_sdk::json_utils::{extract_json_string, json_object_member, write_bytes};

#[derive(Copy, Clone)]
pub(super) struct Mirror {
    pub origin: &'static [u8],
    pub site: &'static [u8],
    pub app: &'static [u8],
}

// The published source already defaults to WWW. Keep that choice for existing
// users; expose every upstream mirror without changing stable work/chapter IDs.
pub(super) const DEFAULT: Mirror = MIRRORS[2];
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
        .and_then(|settings| extract_json_string(settings, b"preferredMirror"));
    MIRRORS.iter().copied().find(|mirror| Some(mirror.origin) == value).unwrap_or(DEFAULT)
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
