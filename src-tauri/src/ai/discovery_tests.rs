use super::{discovery::model_names, tests::*, *};
use crate::test_support::MockServer;
use serde_json::json;
use std::{net::TcpListener, time::Duration};

fn endpoint_with_path(endpoint: &str, path: &str) -> String {
    let mut endpoint = Url::parse(endpoint).unwrap();
    endpoint.set_path(path);
    endpoint.to_string()
}

#[tokio::test]
async fn local_compatible_models_use_v1_and_no_auth_for_an_empty_key() {
    let server = MockServer::new(200, json!({"data":[{"id":"local-model:latest"}]}));
    let draft = AiConfig {
        api_key: String::new(),
        model: String::new(),
        ..config(
            AiProtocol::Openai,
            &endpoint_with_path(&server.endpoint, "/"),
        )
    };
    assert_eq!(
        AiService::new().unwrap().list_models(&draft).await.unwrap(),
        ["local-model:latest"]
    );
    let http = server.captured();
    assert!(http.starts_with("GET /v1/models?fixture=yes HTTP/1.1\r\n"));
    assert!(!http.to_ascii_lowercase().contains("authorization:"));
    assert!(http.ends_with("\r\n\r\n"));
}

#[test]
fn model_urls_preserve_origin_proxy_prefix_and_query_for_each_known_suffix() {
    for (path, expected) in [
        ("/", "/v1/models"),
        ("/v1/chat/completions", "/v1/models"),
        ("/v1/responses/", "/v1/models"),
        ("/proxy/v1/", "/proxy/v1/models"),
        ("/proxy/v1/responses", "/proxy/v1/models"),
        ("/proxy/chat/completions/", "/proxy/models"),
    ] {
        let input = Url::parse(&format!(
            "https://example.test:8443{path}?tenant=a%2Fb&scope=fixture"
        ))
        .unwrap();
        let output = models_endpoint(input.clone()).unwrap();
        assert_eq!(output.path(), expected);
        assert_eq!(output.origin(), input.origin());
        assert_eq!(output.query(), input.query());
    }
    for path in [
        "/proxy/chat",
        "/api/chat",
        "/responses/other",
        "/proxy/notresponses",
    ] {
        let error = models_endpoint(
            Url::parse(&format!("https://example.test{path}?key=fixture-private")).unwrap(),
        )
        .unwrap_err();
        assert!(error.contains("基础地址"));
        assert!(!error.contains("fixture-private"));
    }
}

#[test]
fn model_lists_skip_empty_names_and_deduplicate_in_service_order() {
    assert_eq!(
        model_names(&json!({"data":[{"id":"  model-z "}, {"id":"model-a"}, {"id":"model-z"}, {"id":" "}, {"id":"MODEL-A"}], "object":"list"})).unwrap(),
        vec!["model-z", "model-a", "MODEL-A"]
    );
    assert!(model_names(&json!({"data":[]})).unwrap().is_empty());
}

#[test]
fn malformed_lists_and_business_errors_fail_without_returning_partial_models() {
    for value in [
        json!(null),
        json!([]),
        json!({}),
        json!({"data":null}),
        json!({"models":[{"name":"old-format"}]}),
        json!({"data":[null]}),
        json!({"data":[{}]}),
        json!({"data":[{"id":42}]}),
        json!({"data":[{"id":"valid"},{}]}),
        json!({"error":{"message":"fixture-private"},"data":[]}),
    ] {
        let error = model_names(&value).unwrap_err();
        assert!(!error.contains("fixture-private"));
    }
}

#[tokio::test]
async fn fetching_models_without_model_or_prompt_sends_only_get_using_draft_auth() {
    let service = AiService::new().unwrap();
    for (protocol, path, expected, response) in [
        (
            AiProtocol::Openai,
            "/proxy/v1/chat/completions",
            "/proxy/v1/models",
            json!({"data":[{"id":"model-a"}]}),
        ),
        (
            AiProtocol::OpenaiResponses,
            "/proxy/v1/responses/",
            "/proxy/v1/models",
            json!({"data":[{"id":"model-a"}]}),
        ),
    ] {
        let server = MockServer::new(200, response);
        let draft = AiConfig {
            model: String::new(),
            system_prompt: String::new(),
            ..config(protocol, &endpoint_with_path(&server.endpoint, path))
        };
        let before = draft.clone();
        assert_eq!(service.list_models(&draft).await.unwrap(), ["model-a"]);
        assert_eq!(draft, before);
        let http = server.captured();
        assert!(http.starts_with(&format!("GET {expected}?fixture=yes HTTP/1.1\r\n")));
        assert!(http.ends_with("\r\n\r\n"));
        assert!(http.contains("Bearer fake-api-key"));
        assert!(!http.contains("model-a"));
    }
}

#[tokio::test]
async fn custom_model_endpoint_accepts_empty_key_and_preserves_empty_list() {
    let server = MockServer::new(200, json!({"data":[]}));
    let draft = AiConfig {
        api_key: String::new(),
        model: String::new(),
        ..config(
            AiProtocol::OpenaiResponses,
            &endpoint_with_path(&server.endpoint, "/v1/responses"),
        )
    };
    assert!(AiService::new()
        .unwrap()
        .list_models(&draft)
        .await
        .unwrap()
        .is_empty());
    assert!(!server
        .captured()
        .to_ascii_lowercase()
        .contains("authorization:"));
}

#[tokio::test]
async fn unknown_paths_and_invalid_configuration_do_not_request_models() {
    let server = MockServer::new(200, json!({"data":[]}));
    let service = AiService::new().unwrap();
    let draft = config(AiProtocol::Openai, &server.endpoint);
    assert!(service
        .list_models(&draft)
        .await
        .unwrap_err()
        .contains("基础地址"));
    for endpoint in [
        "",
        "https:/example.test/responses",
        "https://user:secret@example.test/responses",
        "https://example.test/responses#secret",
    ] {
        assert!(service
            .list_models(&AiConfig {
                endpoint: endpoint.into(),
                ..draft.clone()
            })
            .await
            .is_err());
    }
    for protocol in [AiProtocol::Openai, AiProtocol::OpenaiResponses] {
        let official = AiConfig {
            api_key: String::new(),
            model: String::new(),
            ..config(protocol, "https://API.OPENAI.COM/v1/responses")
        };
        assert!(service
            .list_models(&official)
            .await
            .unwrap_err()
            .contains("API Key"));
    }
    assert!(server
        .requests
        .recv_timeout(Duration::from_millis(50))
        .is_err());
}

#[tokio::test]
async fn list_http_json_business_and_structure_errors_never_echo_sensitive_content() {
    let service = AiService::new().unwrap();
    let secret = "fixture-sensitive-key-url";
    for (status, body) in [
        (401, secret.into()),
        (429, json!({"error":secret}).to_string()),
        (503, secret.into()),
        (200, secret.into()),
        (
            200,
            json!({"error":{"code":secret,"type":secret,"message":secret}}).to_string(),
        ),
        (200, json!({"data":[{"unexpected":secret}]}).to_string()),
    ] {
        let server = MockServer::raw(status, body);
        let draft = AiConfig {
            api_key: secret.into(),
            ..config(
                AiProtocol::OpenaiResponses,
                &endpoint_with_path(&server.endpoint, "/v1/responses"),
            )
        };
        let error = service.list_models(&draft).await.unwrap_err();
        assert!(!error.contains(secret));
        assert!(!error.contains(&server.endpoint));
        if status != 200 {
            assert!(error.contains(&format!("HTTP {status}")));
        }
        server.captured();
    }
}

#[tokio::test]
async fn list_network_and_invalid_header_failures_hide_url_and_key() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!(
        "http://{}/v1/responses?private=fixture-private",
        listener.local_addr().unwrap()
    );
    drop(listener);
    let mut draft = config(AiProtocol::OpenaiResponses, &endpoint);
    let service = AiService::new().unwrap();
    let error = service.list_models(&draft).await.unwrap_err();
    assert!(!error.contains("fixture-private"));
    assert!(!error.contains(&endpoint));
    draft.api_key = "fixture-private\r\ninvalid-header".into();
    let error = service.list_models(&draft).await.unwrap_err();
    assert!(!error.contains("fixture-private"));
    assert!(!error.contains(&endpoint));
}

#[tokio::test]
async fn model_redirects_are_reported_without_contacting_destination() {
    let destination = MockServer::new(200, json!({"data":[]}));
    let (endpoint, server) = redirect_server(&destination.endpoint);
    let endpoint = endpoint_with_path(&endpoint, "/v1/chat/completions");
    let error = AiService::new()
        .unwrap()
        .list_models(&config(AiProtocol::Openai, &endpoint))
        .await
        .unwrap_err();
    assert!(error.contains("HTTP 302"));
    assert!(!error.contains(&endpoint));
    assert!(!error.contains(&destination.endpoint));
    server.join().unwrap();
    assert!(destination
        .requests
        .recv_timeout(Duration::from_millis(50))
        .is_err());
}
