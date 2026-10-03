pub const DEFAULT_REGISTRY: &str = "https://registry.npmjs.org";

#[derive(Debug, serde::Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct UpmToolConfig {
    pub registry_url: String,
    pub dist_url: String,
}

impl Default for UpmToolConfig {
    fn default() -> Self {
        Self {
            registry_url: DEFAULT_REGISTRY.into(),
            dist_url: "{registry}/{package}/-/{package_without_scope}-{version}.tgz".into(),
        }
    }
}
