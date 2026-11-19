use super::{
    tests::{captured_body, config, request},
    AiService,
};
use crate::{
    application,
    model::{AiProtocol, Settings},
    storage::Store,
    test_support::{channel_collection, MockServer},
};
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tokio::{sync::mpsc, time::timeout};

fn event(value: Value) -> String {
    format!("data: {value}\r\n\r\n")
}

fn piece(protocol: AiProtocol, text: &str) -> String {
    match protocol {
        AiProtocol::Openai => event(json!({"choices":[{"index":0,"delta":{"content":text}}]})),
        AiProtocol::OpenaiResponses => {
            event(json!({"type":"response.output_text.delta","delta":text}))
        }
    }
}

fn completed(protocol: AiProtocol, text: &str) -> String {
    match protocol {
        AiProtocol::Openai => "data: [DONE]\r\n\r\n".into(),
        AiProtocol::OpenaiResponses => event(json!({"type":"response.completed","response":{
            "status":"completed","output":[{"type":"message","role":"assistant","status":"completed",
            "content":[{"type":"output_text","text":text}]}]}})),
    }
}

#[tokio::test]
async fn both_protocols_deliver_live_utf8_deltas_before_the_completed_history_entry() {
    for protocol in [AiProtocol::Openai, AiProtocol::OpenaiResponses] {
        let metadata = match protocol {
            AiProtocol::Openai => event(
                json!({"choices":[{"index":1,"delta":{"content":"other"}},{"index":0,"delta":{"role":"assistant","reasoning_content":"internal","tool_calls":[]}}]}),
            ),
            AiProtocol::OpenaiResponses => {
                event(json!({"type":"response.reasoning_summary_text.delta","delta":"internal"}))
            }
        };
        let done = match protocol {
            AiProtocol::Openai => {
                event(json!({"choices":[],"usage":{}}))
                    + &event(
                        json!({"choices":[{"index":0,"finish_reason":null,"content_filter_results":{"fixture":{"filtered":false}},"content_filter_offsets":{"check_offset":2}}]}),
                    )
            }
            AiProtocol::OpenaiResponses => {
                event(json!({"type":"response.output_text.done","text":"你 🌍"}))
            }
        };
        let body = format!(
            "\u{feff}: heartbeat\r\n\r\n{metadata}{}{}{done}{}",
            piece(protocol, "你"),
            piece(protocol, " 🌍"),
            completed(protocol, "你 🌍")
        );
        let cut = body.find('🌍').unwrap() + 2;
        let (server, release) =
            MockServer::split_stream(&body.as_bytes()[..cut], &body.as_bytes()[cut..]);
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(dir.path()).unwrap());
        store
            .save_configuration(
                &Settings::default(),
                &channel_collection(&config(protocol, &server.endpoint)),
            )
            .unwrap();
        let writer = Arc::clone(&store);
        let (tx, mut rx) = mpsc::unbounded_channel();
        let translation = tokio::spawn(async move {
            application::translate(&writer, &AiService::new().unwrap(), &request(), |s| {
                tx.send(s.to_owned()).unwrap();
            })
            .await
        });
        assert_eq!(
            timeout(Duration::from_secs(3), rx.recv())
                .await
                .unwrap()
                .as_deref(),
            Some("你")
        );
        assert!(!translation.is_finished());
        assert_eq!(store.list_history("", 10, 0).unwrap().total, 0);
        release.send(()).unwrap();
        let result = translation.await.unwrap().unwrap();
        assert_eq!(rx.recv().await.as_deref(), Some(" 🌍"));
        assert!(rx.recv().await.is_none());
        assert_eq!(result.translated_text, "你 🌍");
        let history = store.list_history("", 10, 0).unwrap();
        assert_eq!(history.total, 1);
        assert_eq!(history.items[0].translated_text, "你 🌍");
        let http = server.captured();
        assert!(http
            .to_ascii_lowercase()
            .contains("accept: text/event-stream\r\n"));
        let request = captured_body(&http);
        assert_eq!(request["stream"], true);
        if protocol == AiProtocol::OpenaiResponses {
            assert_eq!(request["store"], false);
        }
    }
}

#[tokio::test]
async fn incomplete_refused_or_invalid_streams_never_store_the_partial_translation() {
    for (protocol, ending) in [
        (AiProtocol::Openai, String::new()),
        (
            AiProtocol::Openai,
            event(json!({"choices":[{"index":"0","delta":{"content":"bad"}}]}))
                + &completed(AiProtocol::Openai, ""),
        ),
        (
            AiProtocol::Openai,
            event(json!({"choices":[{"delta":{"role":42}}]})) + &completed(AiProtocol::Openai, ""),
        ),
        (
            AiProtocol::Openai,
            event(json!({"choices":[{"delta":42,"finish_reason":"stop"}]}))
                + &completed(AiProtocol::Openai, ""),
        ),
        (
            AiProtocol::Openai,
            event(json!({"choices":[{"delta":{},"finish_reason":"length"}]})),
        ),
        (
            AiProtocol::Openai,
            event(json!({"choices":[{"delta":{"refusal":"private text"}}]})),
        ),
        (
            AiProtocol::Openai,
            "event: error\ndata: {\"message\":\"private text\"}\n\n".into(),
        ),
        (
            AiProtocol::Openai,
            "data: {malformed private text}\n\n".into(),
        ),
        (
            AiProtocol::Openai,
            event(
                json!({"choices":[{"index":0,"finish_reason":null, "content_filter_results":{"fixture":{"filtered":true}}, "content_filter_offsets":{}}]}),
            ) + &completed(AiProtocol::Openai, ""),
        ),
        (
            AiProtocol::Openai,
            event(json!({"choices":[{"index":0,"finish_reason":"content_filter"}]}))
                + &completed(AiProtocol::Openai, ""),
        ),
        (AiProtocol::OpenaiResponses, String::new()),
        (
            AiProtocol::OpenaiResponses,
            event(Value::Null) + &completed(AiProtocol::OpenaiResponses, "half"),
        ),
        (
            AiProtocol::OpenaiResponses,
            event(json!({})) + &completed(AiProtocol::OpenaiResponses, "half"),
        ),
        (
            AiProtocol::OpenaiResponses,
            event(json!({"type":42})) + &completed(AiProtocol::OpenaiResponses, "half"),
        ),
        (
            AiProtocol::OpenaiResponses,
            "event: response.incomplete\ndata: {\"type\":\"some.extension\"}\n\n".to_owned()
                + &completed(AiProtocol::OpenaiResponses, "half"),
        ),
        (AiProtocol::OpenaiResponses, "data: [DONE]\n\n".into()),
        (
            AiProtocol::OpenaiResponses,
            event(json!({"type":"response.incomplete"})),
        ),
        (
            AiProtocol::OpenaiResponses,
            event(json!({"type":"response.failed"})),
        ),
        (
            AiProtocol::OpenaiResponses,
            event(json!({"type":"response.refusal.delta","delta":"private text"})),
        ),
    ] {
        let server = MockServer::events(&(piece(protocol, "half") + &ending));
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let mut preview = String::new();
        let error = application::translate_with_config(
            &store,
            &AiService::new().unwrap(),
            &request(),
            &config(protocol, &server.endpoint),
            |s| preview.push_str(s),
        )
        .await
        .unwrap_err();
        assert_eq!(preview, "half");
        assert!(
            !error.contains("private text")
                && !error.contains("half")
                && !error.contains("fake-api-key")
        );
        assert_eq!(store.list_history("", 10, 0).unwrap().total, 0);
    }
}

#[tokio::test]
async fn chat_accepts_clean_stop_and_rejects_unterminated_data_after_it() {
    let stop = event(json!({"choices":[{"delta":{},"finish_reason":"stop"}]}));
    let body = piece(AiProtocol::Openai, "result") + &stop;
    for (suffix, valid) in [
        ("", true),
        ("data: {", false),
        ("data: {\"error\":true}\n", false),
    ] {
        let server = MockServer::events(&format!("{body}{suffix}"));
        let result = AiService::new()
            .unwrap()
            .translate(&request(), &config(AiProtocol::Openai, &server.endpoint))
            .await;
        assert_eq!(result.is_ok(), valid);
    }
}

#[tokio::test]
async fn active_generation_outlives_the_idle_deadline_but_heartbeats_cannot_keep_a_stalled_request_alive(
) {
    async fn paced(active: bool) -> Result<String, String> {
        let protocol = AiProtocol::Openai;
        let chunk = if active {
            piece(protocol, "A")
        } else {
            ": heartbeat\r\n\r\nevent: ping\r\ndata: {}\r\n\r\n".into()
        };
        let mut chunks = vec![chunk.into_bytes(); 6];
        chunks.push(completed(protocol, "").into_bytes());
        let count = chunks.len();
        let (server, release) =
            MockServer::stream_steps(piece(protocol, "first").as_bytes(), chunks);
        let response = reqwest::get(&server.endpoint).await.unwrap();
        let writer = tokio::spawn(async move {
            for _ in 0..count {
                tokio::time::sleep(Duration::from_millis(125)).await;
                if release.send(()).is_err() {
                    break;
                }
            }
        });
        let result =
            super::stream::read(response, protocol, Duration::from_millis(650), |_| {}).await;
        writer.await.unwrap();
        result
    }
    let (active, stalled) = tokio::join!(paced(true), paced(false));
    assert_eq!(active.unwrap(), "firstAAAAAA");
    assert!(stalled.unwrap_err().contains("超时"));
}

#[tokio::test]
async fn ordinary_json_cannot_be_mistaken_for_a_completed_stream() {
    for protocol in [AiProtocol::Openai, AiProtocol::OpenaiResponses] {
        let server = MockServer::new(200, json!({"output_text":"private response"}));
        let error = AiService::new()
            .unwrap()
            .test(&config(protocol, &server.endpoint))
            .await
            .unwrap_err();
        assert!(error.contains("流式"));
        assert!(!error.contains("private"));
        assert_eq!(captured_body(&server.captured())["stream"], true);
    }
}
