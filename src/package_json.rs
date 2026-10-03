use proto_pdk::{AnyResult, VirtualPath};
use starbase_utils::fs;
use starbase_utils::json::{self, JsonMap, JsonValue};

const NAME: &str = "upm";

pub fn version_candidates(package_json: &JsonValue, path: &VirtualPath) -> AnyResult<Vec<String>> {
    let mut candidates = vec![];

    candidates.extend(dev_engine_version(package_json));
    candidates.extend(package_manager_version(package_json));
    candidates.extend(volta_version(package_json, path)?);
    candidates.extend(engine_version(package_json));

    Ok(candidates)
}

fn dev_engine_version(package_json: &JsonValue) -> Option<String> {
    let field = package_json.pointer("/devEngines/packageManager")?;
    let entries = match field {
        JsonValue::Array(list) => list.iter().collect(),
        single => vec![single],
    };

    entries
        .into_iter()
        .find(|entry| is_upm(entry))
        .and_then(version_of)
}

fn package_manager_version(package_json: &JsonValue) -> Option<String> {
    let field = package_json.get("packageManager")?.as_str()?;
    let (name, version) = field.split_once('@').unwrap_or((field, "latest"));

    // Corepack appends `+<hash>` build metadata.
    (name == NAME).then(|| version.split('+').next().unwrap_or(version).to_owned())
}

fn volta_version(package_json: &JsonValue, path: &VirtualPath) -> AnyResult<Option<String>> {
    let Some(volta) = package_json.get("volta") else {
        return Ok(None);
    };

    if let Some(version) = volta.get(NAME).and_then(JsonValue::as_str) {
        return Ok(Some(version.into()));
    }

    if let Some(extends) = volta.get("extends").and_then(JsonValue::as_str)
        && let Some(parent) = path.parent()
    {
        let extends_path = parent.join(extends);

        if let Ok(content) = fs::read_file(&extends_path)
            && let Ok(other) = json::parse::<JsonValue>(content)
        {
            return volta_version(&other, &extends_path);
        }
    }

    Ok(None)
}

fn engine_version(package_json: &JsonValue) -> Option<String> {
    package_json
        .pointer(&format!("/engines/{NAME}"))?
        .as_str()
        .map(str::to_owned)
}

pub fn pin_dev_engine(package_json: &mut JsonValue, version: &str) {
    let Some(root) = package_json.as_object_mut() else {
        return;
    };

    let dev_engines = root.entry("devEngines").or_insert_with(empty_object);

    if !dev_engines.is_object() {
        *dev_engines = empty_object();
    }

    let field = dev_engines
        .as_object_mut()
        .unwrap()
        .entry("packageManager")
        .or_insert_with(empty_object);
    let named_upm = is_upm(field);

    match field {
        JsonValue::Array(list) => match list.iter_mut().find(|entry| is_upm(entry)) {
            Some(entry) => entry["version"] = version.into(),
            None => list.push(upm_entry(version)),
        },
        JsonValue::Object(object) if object.is_empty() || named_upm => {
            object.insert("name".into(), NAME.into());
            object.insert("version".into(), version.into());
        }
        other => {
            let previous = other.take();
            let mut list = vec![];

            if previous.is_object() {
                list.push(previous);
            }

            list.push(upm_entry(version));
            *other = JsonValue::Array(list);
        }
    }
}

pub fn unpin_dev_engine(package_json: &mut JsonValue) -> Option<String> {
    let dev_engines = package_json.get_mut("devEngines")?.as_object_mut()?;
    let field = dev_engines.get_mut("packageManager")?;
    let mut removed = None;

    let now_empty = match field {
        JsonValue::Array(list) => {
            if let Some(index) = list.iter().position(is_upm) {
                removed = version_of(&list.remove(index));
            }

            list.is_empty()
        }
        single if is_upm(single) => {
            removed = version_of(single);
            true
        }
        _ => false,
    };

    if now_empty {
        dev_engines.remove("packageManager");
    }

    if dev_engines.is_empty() {
        package_json.as_object_mut()?.remove("devEngines");
    }

    removed
}

fn is_upm(entry: &JsonValue) -> bool {
    entry.get("name").and_then(JsonValue::as_str) == Some(NAME)
}

fn version_of(entry: &JsonValue) -> Option<String> {
    entry.get("version")?.as_str().map(str::to_owned)
}

fn empty_object() -> JsonValue {
    JsonValue::Object(JsonMap::new())
}

fn upm_entry(version: &str) -> JsonValue {
    JsonValue::Object(JsonMap::from_iter([
        ("name".to_owned(), NAME.into()),
        ("version".to_owned(), version.into()),
    ]))
}
