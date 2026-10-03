use proto_pdk_test_utils::*;

mod upm_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn rejects_exact_versions_below_the_name_reuse_floor() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("upm-test").await;

        let error = plugin
            .tool
            .plugin
            .call_func_with::<_, _, DownloadPrebuiltOutput>(
                PluginFunction::DownloadPrebuilt,
                DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("1.0.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .await
            .unwrap_err()
            .to_string();

        assert!(error.contains("1.0.0"), "{error}");
        assert!(error.contains("1.0.1"), "{error}");
    }
}
