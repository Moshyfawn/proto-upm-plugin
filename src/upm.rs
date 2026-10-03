use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use proto_pdk::{
    AnyResult, Version, anyhow, command_exists, exec_captured, get_host_env_var,
    get_host_environment,
};

// Versions 0.0.1 to 1.0.0 on the npm name `upm` belong to unrelated packages.
pub fn name_reuse_floor() -> Version {
    Version::new(1, 0, 1)
}

pub fn at_or_above_floor(version: &str) -> bool {
    Version::parse(version).is_ok_and(|v| v >= name_reuse_floor())
}

// proto compares lowercase hex, so the SRI's base64 digest must be re-encoded.
pub fn sri_to_sha512_hex(integrity: &str) -> AnyResult<String> {
    let digest = integrity
        .split_whitespace()
        .find_map(|entry| entry.strip_prefix("sha512-"))
        .ok_or_else(|| anyhow!("No sha512 entry in integrity <id>{integrity}</id>."))?;

    Ok(STANDARD
        .decode(digest)?
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

pub fn node_floor() -> Version {
    Version::new(22, 3, 0)
}

pub fn detect_node_version() -> Option<Version> {
    let raw = get_host_env_var("PROTO_NODE_VERSION")
        .ok()
        .flatten()
        .or_else(|| {
            // A missing command is a host error the guest cannot catch, so probe first.
            command_exists(get_host_environment().ok()?, "node").then_some(())?;

            exec_captured("node", ["--version"])
                .ok()
                .filter(|output| output.exit_code == 0)
                .map(|output| output.stdout)
        })?;

    Version::parse(raw.trim().trim_start_matches('v')).ok()
}
