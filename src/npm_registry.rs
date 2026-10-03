use rustc_hash::FxHashMap;
use serde::Deserialize;

#[derive(Default, Deserialize)]
pub struct Dist {
    pub integrity: Option<String>,
}

#[derive(Deserialize)]
pub struct RegistryVersion {
    pub version: String,
    #[serde(default)]
    pub dist: Dist,
}

#[derive(Deserialize)]
pub struct Packument {
    #[serde(default, rename = "dist-tags")]
    pub dist_tags: FxHashMap<String, String>,
    pub versions: FxHashMap<String, RegistryVersion>,
}
