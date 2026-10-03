use crate::config::UpmToolConfig;
use crate::npm_registry::{Packument, RegistryVersion};
use crate::package_json::{pin_dev_engine, unpin_dev_engine, version_candidates};
use crate::upm::{
    at_or_above_floor, detect_node_version, name_reuse_floor, node_floor, sri_to_sha512_hex,
};
use extism_pdk::*;
use proto_pdk::*;
use starbase_utils::fs;
use starbase_utils::json::{self, JsonValue};
use std::path::PathBuf;

const PACKAGE: &str = "upm";
const BASH_WRAPPER_TEMPLATE: &str = include_str!("../templates/bash-wrapper.sh");
const CMD_WRAPPER_TEMPLATE: &str = include_str!("../templates/cmd-wrapper.cmd");

#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<HostLogInput>);
    fn send_request(input: Json<SendRequestInput>) -> Json<SendRequestOutput>;
}

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    Ok(Json(RegisterToolOutput {
        name: PACKAGE.into(),
        type_of: PluginType::DependencyManager,
        lock_options: ToolLockOptions {
            ignore_os_arch: true,
            ..Default::default()
        },
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        requires: vec!["node".into()],
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn define_tool_config(_: ()) -> FnResult<Json<DefineToolConfigOutput>> {
    Ok(Json(DefineToolConfigOutput {
        schema: schematic::SchemaBuilder::build_root::<UpmToolConfig>(),
    }))
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec!["package.json".into()],
        ignore: vec!["node_modules".into()],
    }))
}

#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<ParseVersionFileInput>,
) -> FnResult<Json<ParseVersionFileOutput>> {
    let mut output = ParseVersionFileOutput::default();

    if input.file != "package.json" {
        return Ok(Json(output));
    }

    let Ok(package_json) = json::parse::<JsonValue>(&input.content) else {
        return Ok(Json(output));
    };

    let mut first_error = None;

    for candidate in version_candidates(&package_json, &input.path)? {
        match UnresolvedVersionSpec::parse(&candidate) {
            Ok(version) => {
                output.version = Some(version);
                break;
            }
            Err(error) => {
                first_error.get_or_insert(error);
            }
        }
    }

    if output.version.is_none()
        && let Some(error) = first_error
    {
        return Err(plugin_err!("{error}"));
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn pin_version(Json(input): Json<PinVersionInput>) -> FnResult<Json<PinVersionOutput>> {
    let mut output = PinVersionOutput::default();
    let file = input.dir.join("package.json");

    let Some(mut package_json) = read_package_json(&file, &mut output.error)? else {
        return Ok(Json(output));
    };

    pin_dev_engine(&mut package_json, &input.version.to_string());
    json::write_file_with_config(&file, &package_json, true)?;

    output.pinned = true;
    output.file = Some(file);

    Ok(Json(output))
}

#[plugin_fn]
pub fn unpin_version(Json(input): Json<UnpinVersionInput>) -> FnResult<Json<UnpinVersionOutput>> {
    let mut output = UnpinVersionOutput::default();
    let file = input.dir.join("package.json");

    let Some(mut package_json) = read_package_json(&file, &mut output.error)? else {
        return Ok(Json(output));
    };

    if let Some(version) = unpin_dev_engine(&mut package_json) {
        let version = UnresolvedVersionSpec::parse(&version)?;
        json::write_file_with_config(&file, &package_json, true)?;

        output.unpinned = true;
        output.version = Some(version);
        output.file = Some(file);
    }

    Ok(Json(output))
}

fn read_package_json(
    file: &VirtualPath,
    error: &mut Option<String>,
) -> AnyResult<Option<JsonValue>> {
    if !file.exists() {
        *error = Some("No <file>package.json</file> exists in the target directory.".into());
        return Ok(None);
    }

    Ok(Some(json::read_file(file)?))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let mut output = LoadVersionsOutput::default();
    let config = get_tool_config::<UpmToolConfig>()?;
    let packument = fetch_packument(config.registry_url())?;

    for item in packument.versions.values() {
        if at_or_above_floor(&item.version) {
            output.versions.push(VersionSpec::parse(&item.version)?);
        }
    }

    for (alias, version) in packument.dist_tags {
        if !at_or_above_floor(&version) {
            continue;
        }

        let version = UnresolvedVersionSpec::parse(&version)?;

        if alias == "latest" {
            output.latest = Some(version.clone());
        }

        output.aliases.entry(alias).or_insert(version);
    }

    if output.latest.is_none() {
        return Err(plugin_err!(
            "The npm <id>latest</id> dist-tag for <id>{PACKAGE}</id> is missing or below the <version>{}</version> floor.",
            name_reuse_floor()
        ));
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let version = &input.context.version;

    if version.is_canary() {
        return Err(plugin_err!(PluginError::UnsupportedCanary {
            tool: PACKAGE.into()
        }));
    }

    // `proto run upm@1.0.0` skips list validation, so the floor is enforced here too.
    if version.as_version().is_none_or(|v| *v < name_reuse_floor()) {
        return Err(plugin_err!(
            "<id>{PACKAGE}</id> <version>{version}</version> belongs to an unrelated package; versions start at <version>{}</version>.",
            name_reuse_floor()
        ));
    }

    let config = get_tool_config::<UpmToolConfig>()?;
    let registry_url = config.registry_url();
    let version = version.to_string();
    let filename = format!("{PACKAGE}-{version}.tgz");
    let integrity = fetch_integrity(registry_url, &version)?;

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some("package".into()),
        checksum: Some(Checksum::sha512(sri_to_sha512_hex(&integrity)?)),
        download_url: config
            .dist_url
            .replace("{registry}", registry_url)
            .replace("{package}", PACKAGE)
            .replace("{package_without_scope}", PACKAGE)
            .replace("{version}", &version)
            .replace("{file}", &filename),
        ..Default::default()
    }))
}

fn fetch_packument(registry_url: &str) -> AnyResult<Packument> {
    fetch_json(format!("{registry_url}/{PACKAGE}/"))
}

fn fetch_integrity(registry_url: &str, version: &str) -> AnyResult<String> {
    let url = format!("{registry_url}/{PACKAGE}/{version}");
    let response = send_request!(&url);

    let document: RegistryVersion = match response.status {
        // Some mirrors only serve the packument.
        404 => fetch_packument(registry_url)?
            .versions
            .remove(version)
            .ok_or_else(|| {
                anyhow!("<id>{PACKAGE}</id> <version>{version}</version> is not in the registry.")
            })?,
        200..300 => response.json()?,
        status => return Err(anyhow!("Failed to request <url>{url}</url> ({status})")),
    };

    document.dist.integrity.ok_or_else(|| {
        anyhow!("The registry has no integrity hash for <id>{PACKAGE}</id> <version>{version}</version>.")
    })
}

#[plugin_fn]
pub fn locate_executables(
    Json(input): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;
    let config = get_tool_config::<UpmToolConfig>()?;
    let mut output = LocateExecutablesOutput::default();

    let upm = write_wrapper(env, &input.install_dir, "shims", "upm")?;
    output.exes.insert("upm".into(), wrapper_config(upm, true));

    // upx gets its own dir so `proto activate` only sees it when enabled.
    if config.upx_shim {
        let upx = write_wrapper(env, &input.install_dir, "upx", "upx")?;
        output.exes_dirs.push(upx.parent().unwrap().to_path_buf());
        output.exes.insert("upx".into(), wrapper_config(upx, false));
    }

    Ok(Json(output))
}

fn wrapper_config(path: PathBuf, primary: bool) -> ExecutableConfig {
    ExecutableConfig {
        exe_path: Some(path),
        no_bin: true,
        primary,
        update_perms: true,
        ..Default::default()
    }
}

fn write_wrapper(
    env: &HostEnvironment,
    install_dir: &VirtualPath,
    dir: &str,
    bin: &str,
) -> AnyResult<PathBuf> {
    let (file, template, bin_path) = if env.os.is_windows() {
        (
            format!("{bin}.cmd"),
            CMD_WRAPPER_TEMPLATE,
            format!("..\\dist\\{bin}.mjs"),
        )
    } else {
        (
            bin.to_string(),
            BASH_WRAPPER_TEMPLATE,
            format!("../dist/{bin}.mjs"),
        )
    };
    let path = PathBuf::from(dir).join(file);

    fs::write_file(
        install_dir.join(&path),
        template.replace("{bin_path}", &bin_path),
    )?;

    Ok(path)
}

#[plugin_fn]
pub fn post_install(Json(_): Json<InstallHook>) -> FnResult<()> {
    if let Some(version) = detect_node_version()
        && version < node_floor()
    {
        host_log!(
            warn,
            "upm requires Node.js {} or newer, but found {version}. On older versions upm exits without printing anything.",
            node_floor()
        );
    }

    Ok(())
}
