use rustc_hash::FxHashMap;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RegistryVersion {
    pub version: String,
}

#[derive(Deserialize)]
pub struct Packument {
    #[serde(default, rename = "dist-tags")]
    pub dist_tags: FxHashMap<String, String>,
    pub versions: FxHashMap<String, RegistryVersion>,
}
