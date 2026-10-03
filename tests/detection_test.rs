use proto_pdk_test_utils::*;

fn parse_input(
    sandbox: &ProtoWasmSandbox,
    plugin: &WasmTestWrapper,
    content: &str,
) -> ParseVersionFileInput {
    ParseVersionFileInput {
        content: content.into(),
        file: "package.json".into(),
        path: plugin
            .tool
            .to_virtual_path(sandbox.path().join("package.json")),
        ..Default::default()
    }
}

mod upm_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn detects_the_version_from_package_json_fields() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file("base/package.json", r#"{ "volta": { "upm": "1.2.0" } }"#);
        let plugin = sandbox.create_plugin("upm-test").await;

        let cases: Vec<(&str, &str, Option<&str>)> = vec![
            (
                "devEngines.packageManager object",
                r#"{ "devEngines": { "packageManager": { "name": "upm", "version": "1.4.0" } } }"#,
                Some("1.4.0"),
            ),
            (
                "devEngines.packageManager array matched by name",
                r#"{ "devEngines": { "packageManager": [
                    { "name": "pnpm", "version": "10.0.0" },
                    { "name": "upm", "version": "^1.2.0" }
                ] } }"#,
                Some("^1.2.0"),
            ),
            (
                "packageManager",
                r#"{ "packageManager": "upm@1.3.0" }"#,
                Some("1.3.0"),
            ),
            (
                "packageManager with a hash suffix",
                r#"{ "packageManager": "upm@1.3.0+sha512.abc123" }"#,
                Some("1.3.0"),
            ),
            (
                "packageManager for another manager is ignored",
                r#"{ "packageManager": "pnpm@10.0.0", "engines": { "upm": "1.1.0" } }"#,
                Some("1.1.0"),
            ),
            (
                "volta.upm",
                r#"{ "volta": { "upm": "1.2.0" } }"#,
                Some("1.2.0"),
            ),
            (
                "volta.extends is followed",
                r#"{ "volta": { "extends": "./base/package.json" } }"#,
                Some("1.2.0"),
            ),
            (
                "engines.upm",
                r#"{ "engines": { "upm": ">=1.1.0" } }"#,
                Some(">=1.1.0"),
            ),
            (
                "devEngines wins over every other field",
                r#"{
                    "devEngines": { "packageManager": { "name": "upm", "version": "1.4.0" } },
                    "packageManager": "upm@1.3.0",
                    "volta": { "upm": "1.2.0" },
                    "engines": { "upm": "1.1.0" }
                }"#,
                Some("1.4.0"),
            ),
            (
                "packageManager wins over volta and engines",
                r#"{
                    "packageManager": "upm@1.3.0",
                    "volta": { "upm": "1.2.0" },
                    "engines": { "upm": "1.1.0" }
                }"#,
                Some("1.3.0"),
            ),
            (
                "volta wins over engines",
                r#"{ "volta": { "upm": "1.2.0" }, "engines": { "upm": "1.1.0" } }"#,
                Some("1.2.0"),
            ),
            (
                "an unparsable value falls through to the next field",
                r#"{
                    "devEngines": { "packageManager": { "name": "upm", "version": "not a version!!" } },
                    "packageManager": "upm@1.3.0"
                }"#,
                Some("1.3.0"),
            ),
            (
                "no upm field",
                r#"{ "name": "app", "engines": { "node": "22" } }"#,
                None,
            ),
        ];

        for (name, content, expected) in cases {
            let output = plugin
                .parse_version_file(parse_input(&sandbox, &plugin, content))
                .await;
            let expected = expected.map(|v| UnresolvedVersionSpec::parse(v).unwrap());

            assert_eq!(output.version, expected, "case: {name}");
        }
    }
}
