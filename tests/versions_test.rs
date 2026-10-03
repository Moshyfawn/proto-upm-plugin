use proto_pdk_test_utils::*;

mod upm_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn lists_nothing_below_the_name_reuse_floor() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("upm-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;
        let floor = Version::new(1, 0, 1);

        assert!(!output.versions.is_empty());

        for version in &output.versions {
            let version = version
                .as_version()
                .expect("every entry is a concrete version");
            assert!(*version >= floor, "{version} is below the floor");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn latest_matches_the_npm_dist_tag() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("upm-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        let packument: serde_json::Value = reqwest::get("https://registry.npmjs.org/upm/")
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let expected = packument["dist-tags"]["latest"].as_str().unwrap();

        assert_eq!(
            output.latest,
            Some(UnresolvedVersionSpec::parse(expected).unwrap())
        );
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());
    }
}
