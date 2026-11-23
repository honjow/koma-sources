//! Source-owned UI templates. Content, request values and account identities stay canonical.
use alloc::{format, string::{String, ToString}};
use koma_source_sdk::{localization::Catalog, runtime::Context};
use serde_json::{Map, Value};

const METADATA: &[u8] = include_bytes!("../metadata.json");

fn field(catalog: &Catalog<'_>, context: &Context<'_>, object: &mut Map<String, Value>,
    field: &str, key: &str) {
    let Some(literal) = object.get(field).and_then(Value::as_str) else { return; };
    let mut buffer = [0; 1024];
    let translated = catalog.text(context, key, literal, &mut buffer)
        .unwrap_or(literal).to_string();
    object.insert(field.to_string(), Value::String(translated));
}

fn named_items(catalog: &Catalog<'_>, context: &Context<'_>, data: &mut Value,
    array: &str, prefix: &str) {
    let Some(items) = data.get_mut(array).and_then(Value::as_array_mut) else { return; };
    for item in items {
        let Some(object) = item.as_object_mut() else { continue; };
        let id = match object.get("id").and_then(Value::as_str) {
            Some(value) => value.to_string(),
            None if array == "filters" && object.get("type").and_then(Value::as_str) == Some("header") => "header".to_string(),
            None => continue,
        };
        for role in ["name", "label", "title", "description"] {
            field(catalog, context, object, role, &format!("{prefix}.{id}.{role}"));
        }
        let Some(options) = object.get_mut("options").and_then(Value::as_array_mut) else { continue; };
        for option in options {
            // IDs/values, including mirror URLs and empty provider values, never change.
            let Some(option) = option.as_object_mut() else { continue; };
            let Some(value) = option.get("value").and_then(Value::as_str).map(ToString::to_string) else { continue; };
            let key_value = if value.is_empty() { "all" } else { &value };
            let key = format!("{prefix}.{id}.option.{key_value}");
            field(catalog, context, option, "label", &key);
        }
    }
}

/// Called only for the existing UI schemas. No HTTP, settings writes or generic
/// recursive translation of website data is performed. Unknown keys keep literals.
pub(super) fn render(operation: &str, request: &[u8], input: &[u8], output: &mut [u8])
    -> Result<usize, &'static str> {
    let catalog = Catalog::from_metadata(METADATA).map_err(|_| "invalid_ui_catalog")?;
    let context = Context::from_request(request);
    let mut data: Value = serde_json::from_slice(input).map_err(|_| "invalid_ui_schema")?;
    match operation {
        "get_settings" => named_items(&catalog, &context, &mut data, "settings", "settings"),
        "get_listings" => named_items(&catalog, &context, &mut data, "listings", "listings"),
        "get_filters" => named_items(&catalog, &context, &mut data, "filters", "filters"),
        _ => return Err("unsupported_ui_schema"),
    }
    let bytes = serde_json::to_vec(&data).map_err(|_| "invalid_ui_schema")?;
    if bytes.len() > output.len() { return Err("buffer_overflow"); }
    output[..bytes.len()].copy_from_slice(&bytes);
    Ok(bytes.len())
}
