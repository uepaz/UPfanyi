use super::{protocol::*, *};
use crate::test_support::MockServer;
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::{Duration, Instant},
};

pub(super) fn request() -> TranslationRequest {
    TranslationRequest {
        text: "Hello & 你好😀\nworld".into(),
        source_lang: "auto".into(),
        target_lang: "zh".into(),
    }
}

pub(super) fn config(protocol: AiProtocol, endpoint: &str) -> AiConfig {
    AiConfig {
        protocol,
        endpoint: endpoint.into(),
        api_key: "fake-api-key".into(),
        model: "  fixture-model  ".into(),
        system_prompt: "Translate {source_lang} to {target_lang}; {target_lang}.".into(),
    }
}

pub(super) fn success(protocol: AiProtocol) -> Value {
    match protocol {
        AiProtocol::Openai => json!({"choices":[{"delta":{"content":"  你好\n世界  "}}]}),
        AiProtocol::OpenaiResponses => json!({
            "status": "completed",
            "output": [{
                "type": "message", "role": "assistant", "status": "completed",
                "content": [{"type":"output_text", "text":"  你好\n世界  "}],
            }],
        }),
    }
}

pub(super) fn stream_server(protocol: AiProtocol, event: Value) -> MockServer {
    match protocol {
        AiProtocol::Openai => MockServer::events(&format!("data: {event}\n\ndata: [DONE]\n\n")),
        AiProtocol::OpenaiResponses => MockServer::responses(event),
    }
}

async fn fixture_text(protocol: AiProtocol, event: Value) -> Result<String, String> {
    let server = stream_server(protocol, event);
    AiService::new()
        .unwrap()
        .translate(&request(), &config(protocol, &server.endpoint))
        .await
        .map(|r| r.translated_text)
}

pub(super) fn captured_body(http: &str) -> Value {
    serde_json::from_str(http.split_once("\r\n\r\n").unwrap().1).unwrap()
}

#[test]
fn incomplete_drafts_can_be_saved_without_runtime_defaults() {
    let draft = AiConfig::default();
    assert!(draft.model.is_empty());
    assert!(validate_config(&draft).is_ok());
    assert!(ensure_configured(&draft).unwrap_err().contains("模型"));
    let draft = AiConfig {
        endpoint: String::new(),
        ..draft
    };
    assert!(validate_config(&draft).is_ok());
    assert!(ensure_configured(&draft).unwrap_err().contains("服务地址"));
    assert!(draft.endpoint.is_empty());
}

#[test]
fn service_addresses_accept_domains_but_reject_malformed_schemes_and_secrets() {
    for endpoint in [
        "relative/path",
        "file:///fixture-secret",
        "ftp://example.test/chat",
        "https:/example.test/chat",
        "https:example.test/chat",
        "https://exam\nple.test/chat",
        "https://example.test/path with spaces",
        "https://user:password@example.test/chat",
        "https://example.test/chat#fragment",
        "https://example.test/chat#",
        "https://example.test\\path",
        "user:secret@example.test",
        "//example.test",
        "https:////example.test",
        "example.test#fragment",
        "example.test/path with spaces",
    ] {
        let draft = config(AiProtocol::Openai, endpoint);
        assert!(validate_config(&draft).is_err(), "{endpoint}");
        assert!(ensure_configured(&draft).is_err(), "{endpoint}");
    }
    for endpoint in [
        "https://example.test/custom/chat?region=fixture",
        "http://127.0.0.1:11434/api/chat",
        " https://example.test/chat ",
        "relay.example.test",
        "relay.example.test:8443/v1?tenant=fixture",
        "localhost:11434",
        "[::1]:11434",
        "127.0.0.1:11434",
    ] {
        assert!(validate_config(&config(AiProtocol::Openai, endpoint)).is_ok());
    }
}

#[test]
fn bare_domains_default_to_https_without_mutating_configuration() {
    let draft = config(
        AiProtocol::Openai,
        "relay.example.test:8443/v1?tenant=fixture",
    );
    assert_eq!(
        validate_endpoint(&draft.endpoint).unwrap().as_str(),
        "https://relay.example.test:8443/v1?tenant=fixture"
    );
    assert_eq!(draft.endpoint, "relay.example.test:8443/v1?tenant=fixture");
    let official = AiConfig {
        api_key: String::new(),
        ..config(AiProtocol::OpenaiResponses, "API.OPENAI.COM")
    };
    assert!(ensure_configured(&official)
        .unwrap_err()
        .contains("API Key"));
}

#[tokio::test]
async fn service_domains_generate_translation_paths_for_all_interfaces() {
    let service = AiService::new().unwrap();
    for (protocol, base, expected) in [
        (AiProtocol::Openai, "/", "/v1/chat/completions"),
        (AiProtocol::OpenaiResponses, "/", "/v1/responses"),
        (AiProtocol::Openai, "/v1/", "/v1/chat/completions"),
        (
            AiProtocol::OpenaiResponses,
            "/gateway/v1",
            "/gateway/v1/responses",
        ),
    ] {
        let server = stream_server(protocol, success(protocol));
        let mut address = Url::parse(&server.endpoint).unwrap();
        address.set_path(base);
        let draft = config(protocol, address.as_str());
        let result = service.translate(&request(), &draft).await.unwrap();
        assert_eq!(result.translated_text, "你好\n世界");
        let http = server.captured();
        assert!(http.starts_with(&format!("POST {expected}?fixture=yes HTTP/1.1\r\n")));
        assert_eq!(captured_body(&http)["model"], "fixture-model");
    }
}

#[test]
fn official_openai_requires_key_but_other_compatible_hosts_do_not() {
    let mut draft = config(
        AiProtocol::Openai,
        "https://API.OPENAI.COM/v1/chat/completions",
    );
    draft.api_key = " \t ".into();
    assert!(validate_config(&draft).is_ok());
    assert!(ensure_configured(&draft).unwrap_err().contains("API Key"));
    draft.endpoint = "https://gateway.example.test/v1/chat/completions".into();
    assert!(ensure_configured(&draft).is_ok());
    draft.endpoint = "http://localhost:11434".into();
    assert!(ensure_configured(&draft).is_ok());
}

#[test]
fn typed_config_rejects_missing_fields_unknown_protocol_and_old_fields() {
    let value =
        serde_json::to_value(config(AiProtocol::Openai, "https://example.test/chat")).unwrap();
    for field in ["protocol", "endpoint", "apiKey", "model", "systemPrompt"] {
        let mut incomplete = value.clone();
        incomplete.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<AiConfig>(incomplete).is_err());
    }
    for protocol in ["unknown", "ollama"] {
        let mut invalid = value.clone();
        invalid["protocol"] = json!(protocol);
        assert!(serde_json::from_value::<AiConfig>(invalid).is_err());
    }
    let mut invalid = value;
    invalid["apiName"] = json!("removed-provider");
    assert!(serde_json::from_value::<AiConfig>(invalid).is_err());
}

#[test]
fn prompts_use_language_names_and_leave_original_text_unchanged() {
    let body = request_body(
        &request(),
        &config(AiProtocol::Openai, "https://example.test/chat"),
    );
    assert_eq!(
        body["messages"][0]["content"],
        "Translate 自动检测语言 to 中文; 中文."
    );
    assert_eq!(body["messages"][1]["content"], request().text);
    let translated = TranslationRequest {
        source_lang: "ja".into(),
        target_lang: "pt-BR".into(),
        ..request()
    };
    let body = request_body(
        &translated,
        &config(AiProtocol::Openai, "https://example.test/chat"),
    );
    assert_eq!(
        body["messages"][0]["content"],
        "Translate 日语 to pt-BR; pt-BR."
    );
}

#[test]
fn empty_prompt_is_not_replaced_with_a_default() {
    let draft = AiConfig {
        system_prompt: String::new(),
        ..config(AiProtocol::Openai, "https://example.test/chat")
    };
    assert_eq!(
        request_body(&request(), &draft)["messages"][0]["content"],
        ""
    );
}

#[tokio::test]
async fn protocols_use_their_own_parameters_and_response_paths() {
    for protocol in [AiProtocol::Openai, AiProtocol::OpenaiResponses] {
        let body = request_body(&request(), &config(protocol, "https://example.test/chat"));
        assert_eq!(body["model"], "fixture-model");
        assert_eq!(body["stream"], true);
        match protocol {
            AiProtocol::Openai => {
                assert_eq!(body["messages"][0]["role"], "system");
                assert_eq!(body["messages"][1]["role"], "user");
                assert_eq!(body["temperature"], 0.3);
                assert_eq!(body["max_tokens"], 4096);
                assert!(body.get("options").is_none());
            }
            AiProtocol::OpenaiResponses => {
                assert_eq!(
                    body["instructions"],
                    "Translate 自动检测语言 to 中文; 中文."
                );
                assert_eq!(body["input"], request().text);
                assert_eq!(body["store"], false);
                for field in ["messages", "temperature", "max_tokens", "options"] {
                    assert!(body.get(field).is_none());
                }
            }
        }
        assert_eq!(
            fixture_text(protocol, success(protocol)).await.unwrap(),
            "你好\n世界"
        );
        let other_protocol = if protocol == AiProtocol::Openai {
            AiProtocol::OpenaiResponses
        } else {
            AiProtocol::Openai
        };
        assert!(fixture_text(protocol, success(other_protocol))
            .await
            .is_err());
    }
}

#[tokio::test]
async fn malformed_missing_or_empty_responses_are_errors() {
    for protocol in [AiProtocol::Openai, AiProtocol::OpenaiResponses] {
        for value in [json!(null), json!([]), json!({}), json!({"choices": []})] {
            assert!(fixture_text(protocol, value).await.is_err());
        }
        for content in [json!(null), json!([]), json!(42), json!(" \n\t ")] {
            let value = match protocol {
                AiProtocol::Openai => json!({"choices":[{"delta":{"content":content}}]}),
                AiProtocol::OpenaiResponses => json!({"status":"completed", "output":[{
                    "type":"message", "status":"completed", "role":"assistant",
                    "content":[{"type":"output_text", "text":content}],
                }]}),
            };
            assert!(fixture_text(protocol, value).await.is_err());
        }
    }
}

#[tokio::test]
async fn business_errors_never_echo_provider_code_type_or_message() {
    for protocol in [AiProtocol::Openai, AiProtocol::OpenaiResponses] {
        for error in [
            json!("fixture-private-text"),
            json!({"code":"fixture-private-key","type":"fixture-private-url","message":"fixture-private-text"}),
        ] {
            let message = fixture_text(protocol, json!({"error": error}))
                .await
                .unwrap_err();
            assert!(message.contains("业务错误"));
            assert!(!message.contains("fixture-private"));
        }
    }
}

#[tokio::test]
async fn all_interfaces_send_real_local_http_and_return_actual_model_and_utc() {
    let service = AiService::new().unwrap();
    for protocol in [AiProtocol::Openai, AiProtocol::OpenaiResponses] {
        let server = stream_server(protocol, success(protocol));
        let draft = config(protocol, &server.endpoint);
        let result = service.translate(&request(), &draft).await.unwrap();
        assert_eq!(result.source_text, request().text);
        assert_eq!(result.translated_text, "你好\n世界");
        assert_eq!(result.source_lang, "auto");
        assert_eq!(result.target_lang, "zh");
        assert_eq!(result.model, "fixture-model");
        assert!(result.warning.is_none());
        assert!(result.create_time.ends_with('Z'));
        assert_eq!(
            chrono::DateTime::parse_from_rfc3339(&result.create_time)
                .unwrap()
                .offset()
                .local_minus_utc(),
            0
        );
        let http = server.captured();
        assert!(http.starts_with("POST /complete/chat?fixture=yes HTTP/1.1\r\n"));
        assert!(http
            .to_ascii_lowercase()
            .contains("content-type: application/json\r\n"));
        assert!(http.contains("Bearer fake-api-key"));
        let body = captured_body(&http);
        assert_eq!(body, request_body(&request(), &draft));
        assert!(body.get("apiKey").is_none());
        assert!(!server.endpoint.contains("apiKey"));
    }
}

#[tokio::test]
async fn local_compatible_chat_uses_v1_and_no_auth_for_an_empty_key() {
    let server = stream_server(AiProtocol::Openai, success(AiProtocol::Openai));
    let mut address = Url::parse(&server.endpoint).unwrap();
    address.set_path("/");
    let draft = AiConfig {
        api_key: String::new(),
        ..config(AiProtocol::Openai, address.as_str())
    };
    let result = AiService::new()
        .unwrap()
        .translate(&request(), &draft)
        .await
        .unwrap();
    assert_eq!(result.translated_text, "你好\n世界");
    let http = server.captured();
    assert!(http.starts_with("POST /v1/chat/completions?fixture=yes HTTP/1.1\r\n"));
    assert!(!http.to_ascii_lowercase().contains("authorization:"));
    let body = captured_body(&http);
    assert_eq!(body["messages"][1]["content"], request().text);
    assert_eq!(body["stream"], true);
    assert!(body.get("options").is_none());
}

#[tokio::test]
async fn connection_test_uses_an_independent_snapshot_and_fixed_sample() {
    let server = stream_server(AiProtocol::Openai, success(AiProtocol::Openai));
    let draft = config(AiProtocol::Openai, &server.endpoint);
    let before = draft.clone();
    let result = AiService::new().unwrap().test(&draft).await.unwrap();
    assert_eq!(result.source_text, "Hello");
    assert_eq!(result.source_lang, "en");
    assert_eq!(result.target_lang, "zh");
    assert_eq!(draft, before);
    assert_eq!(
        captured_body(&server.captured())["messages"][1]["content"],
        "Hello"
    );
}

#[tokio::test]
async fn invalid_inputs_and_missing_config_do_not_send_http() {
    let server = stream_server(AiProtocol::Openai, success(AiProtocol::Openai));
    let service = AiService::new().unwrap();
    let draft = config(AiProtocol::Openai, &server.endpoint);
    for invalid in [
        TranslationRequest {
            text: " \n ".into(),
            ..request()
        },
        TranslationRequest {
            source_lang: "zh/secret".into(),
            ..request()
        },
        TranslationRequest {
            target_lang: "auto".into(),
            ..request()
        },
    ] {
        assert!(service.translate(&invalid, &draft).await.is_err());
    }
    assert!(service
        .translate(
            &request(),
            &AiConfig {
                model: " ".into(),
                ..draft.clone()
            }
        )
        .await
        .is_err());
    assert!(service
        .test(&AiConfig {
            endpoint: String::new(),
            ..draft
        })
        .await
        .is_err());
    assert!(server
        .requests
        .recv_timeout(Duration::from_millis(50))
        .is_err());
}

#[tokio::test]
async fn http_json_and_business_failures_have_no_sensitive_output() {
    let service = AiService::new().unwrap();
    let secret = "fixture-sensitive-key-original-url";
    for (status, body) in [
        (401, secret.into()),
        (429, json!({"error": secret}).to_string()),
        (503, secret.into()),
        (200, secret.into()),
        (
            200,
            json!({"error":{"code":secret,"type":secret,"message":secret}}).to_string(),
        ),
        (200, json!({"choices":[]}).to_string()),
        (
            200,
            json!({"choices":[{"message":{"content":" \n "}}]}).to_string(),
        ),
    ] {
        let server = MockServer::raw(status, body);
        let draft = AiConfig {
            api_key: secret.into(),
            ..config(AiProtocol::Openai, &server.endpoint)
        };
        let input = TranslationRequest {
            text: secret.into(),
            ..request()
        };
        let error = service.translate(&input, &draft).await.unwrap_err();
        assert!(!error.contains(secret));
        assert!(!error.contains(&server.endpoint));
        if status != 200 {
            assert!(error.contains(&format!("HTTP {status}")));
        }
        server.captured();
    }
}

#[tokio::test]
async fn network_and_invalid_header_errors_do_not_echo_url_or_key() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!(
        "http://{}/chat?private=fixture-private-key",
        listener.local_addr().unwrap()
    );
    drop(listener);
    let mut draft = config(AiProtocol::Openai, &endpoint);
    let service = AiService::new().unwrap();
    let error = service.translate(&request(), &draft).await.unwrap_err();
    assert!(!error.contains(&endpoint));
    assert!(!error.contains("fixture-private-key"));
    draft.api_key = "fixture-private-key\r\ninvalid-header".into();
    let error = service.translate(&request(), &draft).await.unwrap_err();
    assert!(!error.contains("fixture-private-key"));
    assert!(!error.contains(&endpoint));
}

#[tokio::test]
async fn redirect_is_reported_without_contacting_the_destination() {
    let destination = stream_server(AiProtocol::Openai, success(AiProtocol::Openai));
    let (endpoint, server) = redirect_server(&destination.endpoint);
    let error = AiService::new()
        .unwrap()
        .translate(&request(), &config(AiProtocol::Openai, &endpoint))
        .await
        .unwrap_err();
    assert!(error.contains("HTTP 302"));
    server.join().unwrap();
    assert!(destination
        .requests
        .recv_timeout(Duration::from_millis(50))
        .is_err());
    assert!(!error.contains(&endpoint));
    assert!(!error.contains(&destination.endpoint));
}

// 共享 MockServer 固定响应不带 Location；此服务只补充真实 HTTP 重定向场景。
pub(super) fn redirect_server(destination: &str) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/redirect", listener.local_addr().unwrap());
    let destination = destination.to_owned();
    listener.set_nonblocking(true).unwrap();
    let handle = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let (mut socket, _) = loop {
            match listener.accept() {
                Ok(socket) => break socket,
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(5));
                }
                _ => panic!("重定向测试服务未在期限内收到连接"),
            }
        };
        // Windows accept 可能继承非阻塞状态；请求读取由 socket 超时限制。
        socket.set_nonblocking(false).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        socket
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            assert!(Instant::now() < deadline, "重定向测试请求读取超时");
            let count = socket.read(&mut buffer).unwrap();
            assert_ne!(count, 0);
            bytes.extend_from_slice(&buffer[..count]);
            if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                if bytes.len() >= end + 4 + content_length {
                    break;
                }
            }
        }
        write!(socket, "HTTP/1.1 302 Found\r\nLocation: {destination}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
    });
    (endpoint, handle)
}
