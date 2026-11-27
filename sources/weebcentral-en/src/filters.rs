use alloc::{format, vec::Vec};
use serde_json::{json, Value};

use crate::{Labels, Result};

const SORT: &[&str] = &[
    "Best Match",
    "Alphabet",
    "Popularity",
    "Subscribers",
    "Recently Added",
    "Latest Updates",
];
const ORDER: &[&str] = &["Descending", "Ascending"];
const FLAG: &[&str] = &["Any", "True", "False"];
const STATUS: &[&str] = &["Ongoing", "Complete", "Hiatus", "Canceled"];
const TYPES: &[&str] = &["Manga", "Manhwa", "Manhua", "OEL"];
const TAGS: &[&str] = &[
    "Action",
    "Adult",
    "Adventure",
    "Comedy",
    "Doujinshi",
    "Drama",
    "Ecchi",
    "Fantasy",
    "Gender Bender",
    "Harem",
    "Hentai",
    "Historical",
    "Horror",
    "Isekai",
    "Josei",
    "Lolicon",
    "Martial Arts",
    "Mature",
    "Mecha",
    "Mystery",
    "Psychological",
    "Romance",
    "School Life",
    "Sci-fi",
    "Seinen",
    "Shotacon",
    "Shoujo",
    "Shoujo Ai",
    "Shounen",
    "Shounen Ai",
    "Slice of Life",
    "Smut",
    "Sports",
    "Supernatural",
    "Tragedy",
    "Yaoi",
    "Yuri",
    "Other",
];

struct Filter {
    id: &'static str,
    name: &'static str,
    kind: &'static str,
    group: &'static str,
    options: &'static [&'static str],
}

const FILTERS: &[Filter] = &[
    Filter {
        id: "sort",
        name: "Sort by",
        kind: "select",
        group: "sort",
        options: SORT,
    },
    Filter {
        id: "order",
        name: "Sort order",
        kind: "select",
        group: "order",
        options: ORDER,
    },
    Filter {
        id: "official",
        name: "Official translation",
        kind: "select",
        group: "flag",
        options: FLAG,
    },
    Filter {
        id: "anime",
        name: "Anime adaptation",
        kind: "select",
        group: "flag",
        options: FLAG,
    },
    Filter {
        id: "adult",
        name: "Adult content",
        kind: "select",
        group: "flag",
        options: FLAG,
    },
    Filter {
        id: "author",
        name: "Author (case-sensitive)",
        kind: "text",
        group: "",
        options: &[],
    },
    Filter {
        id: "included_status",
        name: "Series status",
        kind: "multiselect",
        group: "status",
        options: STATUS,
    },
    Filter {
        id: "included_type",
        name: "Series type",
        kind: "multiselect",
        group: "type",
        options: TYPES,
    },
    Filter {
        id: "tags",
        name: "Tags",
        kind: "tristate",
        group: "tag",
        options: TAGS,
    },
];

pub(super) fn describe(labels: &Labels<'_>) -> Result<Value> {
    let mut filters = Vec::new();
    for filter in FILTERS {
        let mut item = json!({"id":filter.id,"name":labels.text(&format!("filter.{}",filter.id),filter.name)?,"type":filter.kind});
        if !filter.options.is_empty() {
            let mut options = Vec::new();
            if filter.id == "sort" {
                options.push(json!({"value":"","label":labels.text("option.sort.default","Listing default")?}));
            }
            for (index, value) in filter.options.iter().enumerate() {
                let key = format!("option.{}.{index}", filter.group);
                options.push(json!({"value":value,"label":labels.text(&key,value)?}));
            }
            item["options"] = json!(options);
            if filter.kind == "select" {
                item["default"] = json!(if filter.id == "sort" {
                    ""
                } else {
                    filter.options[0]
                });
            }
        }
        filters.push(item);
    }
    Ok(json!({"filters":filters}))
}

pub(super) fn append<T: url::form_urlencoded::Target>(
    choices: &Value,
    default_sort: &str,
    pairs: &mut url::form_urlencoded::Serializer<'_, T>,
) -> Result<()> {
    for filter in FILTERS {
        let selected = &choices[filter.id];
        match filter.kind {
            "select" => {
                let fallback = if filter.id == "sort" {
                    default_sort
                } else {
                    filter.options[0]
                };
                let value = match selected {
                    Value::Null => fallback,
                    Value::String(s) if s.is_empty() && filter.id == "sort" => fallback,
                    Value::String(s) if filter.options.contains(&s.as_str()) => s,
                    _ => return Err("invalid_request"),
                };
                pairs.append_pair(filter.id, value);
            }
            "text" => match selected {
                Value::Null => {}
                Value::String(s) => {
                    if !s.trim().is_empty() {
                        pairs.append_pair(filter.id, s);
                    }
                }
                _ => return Err("invalid_request"),
            },
            _ if selected.is_null() => {}
            _ => {
                let values = selected.as_array().ok_or("invalid_request")?;
                for option in filter.options {
                    let include = values.iter().any(|v| v.as_str() == Some(option));
                    if filter.kind == "tristate" {
                        let exclude = values
                            .iter()
                            .any(|v| v.as_str().and_then(|s| s.strip_prefix('!')) == Some(option));
                        if include {
                            pairs.append_pair("included_tag", option);
                        } else if exclude {
                            pairs.append_pair("excluded_tag", option);
                        }
                    } else if include {
                        pairs.append_pair(filter.id, option);
                    }
                }
            }
        }
    }
    Ok(())
}
