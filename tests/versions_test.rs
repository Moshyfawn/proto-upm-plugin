use proto_pdk_test_utils::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

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

    #[tokio::test(flavor = "multi_thread")]
    async fn every_dist_tag_above_the_floor_is_an_alias() {
        let registry = FakeRegistry::serve(
            r#"{
                "dist-tags": { "latest": "1.4.0", "next": "2.0.0-beta.1", "old": "1.0.0" },
                "versions": {
                    "0.0.1": { "version": "0.0.1" },
                    "1.0.0": { "version": "1.0.0" },
                    "1.0.1": { "version": "1.0.1" },
                    "1.4.0": { "version": "1.4.0" },
                    "2.0.0-beta.1": { "version": "2.0.0-beta.1" }
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

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        let mut aliases: Vec<_> = output
            .aliases
            .iter()
            .map(|(alias, version)| (alias.as_str(), version.to_string()))
            .collect();
        aliases.sort();

        assert_eq!(
            aliases,
            vec![("latest", "1.4.0".into()), ("next", "2.0.0-beta.1".into())]
        );
        assert_eq!(
            output.latest,
            Some(UnresolvedVersionSpec::parse("1.4.0").unwrap())
        );

        let mut versions: Vec<_> = output.versions.iter().map(|v| v.to_string()).collect();
        versions.sort();

        assert_eq!(versions, vec!["1.0.1", "1.4.0", "2.0.0-beta.1"]);
    }
}

struct FakeRegistry {
    url: String,
}

impl FakeRegistry {
    async fn serve(packument: &'static str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());

        tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0u8; 4096];
                let read = socket.read(&mut request).await.unwrap_or(0);
                let request = String::from_utf8_lossy(&request[..read]);
                let path = request.split_whitespace().nth(1).unwrap_or("");

                let (status, body) = if path == "/upm/" {
                    ("200 OK", packument)
                } else {
                    ("404 Not Found", r#"{"error":"Not found"}"#)
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
            }
        });

        Self { url }
    }

    fn tool_config(&self) -> serde_json::Value {
        serde_json::json!({ "registry-url": self.url })
    }
}
