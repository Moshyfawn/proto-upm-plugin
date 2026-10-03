use proto_pdk::Version;

// Versions 0.0.1 to 1.0.0 on the npm name `upm` belong to unrelated packages.
pub fn name_reuse_floor() -> Version {
    Version::new(1, 0, 1)
}

pub fn at_or_above_floor(version: &str) -> bool {
    Version::parse(version).is_ok_and(|v| v >= name_reuse_floor())
}
