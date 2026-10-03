pub const DEFAULT_REGISTRY: &str = "https://registry.npmjs.org";

#[derive(Debug, serde::Deserialize)]
#[cfg_attr(feature = "wasm", derive(schematic::Schematic))]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct UpmToolConfig {
    pub registry_url: String,
    pub dist_url: String,
    pub upx_shim: bool,
}

impl Default for UpmToolConfig {
    fn default() -> Self {
        Self {
            registry_url: DEFAULT_REGISTRY.into(),
            dist_url: "{registry}/{package}/-/{package_without_scope}-{version}.tgz".into(),
            upx_shim: true,
        }
    }
}

impl UpmToolConfig {
    pub fn registry_url(&self) -> &str {
        self.registry_url.trim_end_matches('/')
    }
}
