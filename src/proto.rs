use crate::config::UpmToolConfig;
use crate::npm_registry::Packument;
use crate::upm::{at_or_above_floor, name_reuse_floor};
use extism_pdk::*;
use proto_pdk::*;
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::path::PathBuf;

const PACKAGE: &str = "upm";
const BASH_WRAPPER_TEMPLATE: &str = include_str!("../templates/bash-wrapper.sh");
const CMD_WRAPPER_TEMPLATE: &str = include_str!("../templates/cmd-wrapper.cmd");

#[host_fn]
extern "ExtismHost" {
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
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec!["package.json".into()],
        ignore: vec!["node_modules".into()],
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let mut output = LoadVersionsOutput::default();
    let config = get_tool_config::<UpmToolConfig>()?;
    let registry_url = config.registry_url.trim_end_matches('/');

    let packument: Packument = fetch_json(format!("{registry_url}/{PACKAGE}/"))?;

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
    let config = get_tool_config::<UpmToolConfig>()?;
    let filename = format!("{PACKAGE}-{version}.tgz");

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some("package".into()),
        download_url: config
            .dist_url
            .replace("{registry}", config.registry_url.trim_end_matches('/'))
            .replace("{package}", PACKAGE)
            .replace("{package_without_scope}", PACKAGE)
            .replace("{version}", &version.to_string())
            .replace("{file}", &filename),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(input): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;
    let upm = write_wrapper(env, &input.install_dir, "shims", "upm")?;

    Ok(Json(LocateExecutablesOutput {
        exes: FxHashMap::from_iter([("upm".to_string(), wrapper_config(upm, true))]),
        ..Default::default()
    }))
}

// Wrappers are scripts, not real binaries, so they are never bin-linked.
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
