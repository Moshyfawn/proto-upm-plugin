mod support;

use proto_pdk_test_utils::*;
use support::FakeRegistry;

const UPM_1_0_1_SHA512: &str = "082b55906c279c756a30783745dee53deb6fb625c7fffabed59f1b69554450bca7cb1fa4a4ea2bc72f55976a9fc226192db328b253c13b96f59c05d1171e78d5";

fn download_input(version: &str) -> DownloadPrebuiltInput {
    DownloadPrebuiltInput {
        context: PluginContext {
            version: VersionSpec::parse(version).unwrap(),
            ..Default::default()
        },
        ..Default::default()
    }
}

mod upm_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn resolves_the_tarball_url_and_sha512_checksum() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("upm-test").await;

        let output = plugin.download_prebuilt(download_input("1.0.1")).await;

        assert_eq!(
            output.download_url,
            "https://registry.npmjs.org/upm/-/upm-1.0.1.tgz"
        );
        assert_eq!(output.archive_prefix.as_deref(), Some("package"));
        assert_eq!(
            output.checksum,
            Some(Checksum::sha512(UPM_1_0_1_SHA512.into()))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn falls_back_to_the_packument_when_the_version_document_is_missing() {
        let registry = FakeRegistry::serve(
            r#"{
                "dist-tags": { "latest": "1.0.1" },
                "versions": {
                    "1.0.1": {
                        "version": "1.0.1",
                        "dist": { "integrity": "sha512-CCtVkGwnnHVqMHg3Rd7lPetvtiXH//q+1Z8baVVEULynyx+kpOorxy9Vl2qfwiYZLbMoslPBO5b1nAXRFx541Q==" }
                    }
                }
            }"#,
        )
        .await;
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("upm-test", |config| {
                config.tool_config(registry.tool_config());
            })
            .await;

        let output = plugin.download_prebuilt(download_input("1.0.1")).await;

        assert_eq!(
            output.download_url,
            format!("{}/upm/-/upm-1.0.1.tgz", registry.url)
        );
        assert_eq!(
            output.checksum,
            Some(Checksum::sha512(UPM_1_0_1_SHA512.into()))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn rejects_exact_versions_below_the_name_reuse_floor() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("upm-test").await;

        let error = plugin
            .tool
            .plugin
            .call_func_with::<_, _, DownloadPrebuiltOutput>(
                PluginFunction::DownloadPrebuilt,
                download_input("1.0.0"),
            )
            .await
            .unwrap_err()
            .to_string();

        assert!(error.contains("1.0.0"), "{error}");
        assert!(error.contains("1.0.1"), "{error}");
    }
}
