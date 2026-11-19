use super::{protocol::*, tests::*, *};
use crate::test_support::MockServer;
use serde_json::json;

#[test]
fn responses_concatenates_completed_assistant_text_in_order_and_skips_non_message_items() {
    let value = json!({
        "status": "completed",
        "output_text": "SDK-only fallback must not be read",
        "output": [
            {"type":"reasoning", "summary":[{"type":"summary_text", "text":"private reasoning"}]},
            {"type":"message", "role":"assistant", "status":"completed", "content":[
                {"type":"output_text", "text":"  第一段", "annotations":[]},
                {"type":"output_text", "text":"\n第二段"},
            ]},
            {"type":"function_call", "arguments":"private tool arguments"},
            {"type":"message", "role":"assistant", "status":"completed", "content":[
                {"type":"output_text", "text":"\n第三段  "},
            ]},
        ],
    });
    assert_eq!(
        completed_response_text(&value).unwrap(),
        "第一段\n第二段\n第三段"
    );
}

#[test]
fn responses_rejects_missing_non_completed_and_invalid_status_without_partial_text() {
    for status in [
        json!(null),
        json!(false),
        json!("incomplete"),
        json!("failed"),
        json!("in_progress"),
        json!("queued"),
        json!("cancelled"),
    ] {
        let mut value = success(AiProtocol::OpenaiResponses);
        value["status"] = status;
        value["output"][0]["content"][0]["text"] = json!("fixture-private-partial");
        let error = completed_response_text(&value).unwrap_err();
        assert!(!error.contains("fixture-private"));
    }
    let mut value = success(AiProtocol::OpenaiResponses);
    value.as_object_mut().unwrap().remove("status");
    assert!(completed_response_text(&value).is_err());
}

#[test]
fn responses_rejects_refusals_and_malformed_assistant_output() {
    let invalid_outputs = [
        json!(null),
        json!({}),
        json!([null]),
        json!([{}]),
        json!([{"type":"message", "role":"assistant", "status":"incomplete", "content":[]}]),
        json!([{"type":"message", "role":"assistant", "content":[]}]),
        json!([{"type":"message", "role":"user", "status":"completed", "content":[]}]),
        json!([{"type":"message", "role":"assistant", "status":"completed", "content":null}]),
        json!([{"type":"message", "role":"assistant", "status":"completed", "content":[null]}]),
        json!([{"type":"message", "role":"assistant", "status":"completed", "content":[{"type":"output_text", "text":42}]}]),
        json!([{"type":"message", "role":"assistant", "status":"completed", "content":[{"type":"other", "text":"fixture-private"}]}]),
        json!([{"type":"message", "role":"assistant", "status":"completed", "content":[
            {"type":"output_text", "text":"partial"},
            {"type":"refusal", "refusal":"fixture-private-reason"},
        ]}]),
    ];
    for output in invalid_outputs {
        let value = json!({"status":"completed", "output":output});
        let error = completed_response_text(&value).unwrap_err();
        assert!(!error.contains("fixture-private"));
    }
    for output in [
        json!([]),
        json!([{"type":"reasoning", "summary":[]}]),
        json!([{"type":"message", "role":"assistant", "status":"completed", "content":[]}]),
    ] {
        assert!(
            completed_response_text(&json!({"status":"completed", "output":output}))
                .unwrap_err()
                .contains("空译文")
        );
    }
}

#[tokio::test]
async fn responses_uses_snapshot_instructions_and_never_stores_response_server_side() {
    let server = MockServer::responses(success(AiProtocol::OpenaiResponses));
    let draft = config(AiProtocol::OpenaiResponses, &server.endpoint);
    let before = draft.clone();
    let result = AiService::new().unwrap().test(&draft).await.unwrap();
    assert_eq!(draft, before);
    assert_eq!(result.source_text, "Hello");
    let body = captured_body(&server.captured());
    assert_eq!(body["model"], "fixture-model");
    assert_eq!(body["input"], "Hello");
    assert_eq!(body["instructions"], "Translate 英语 to 中文; 中文.");
    assert_eq!(body["stream"], true);
    assert_eq!(body["store"], false);
    assert_eq!(body.as_object().unwrap().len(), 5);
    let mut empty_prompt = draft;
    empty_prompt.system_prompt.clear();
    assert_eq!(request_body(&request(), &empty_prompt)["instructions"], "");
}

#[tokio::test]
async fn responses_failures_from_local_http_do_not_expose_partial_results_or_secrets() {
    for value in [
        json!({"status":"incomplete", "output":[], "incomplete_details":{"reason":"fixture-private"}}),
        json!({"status":"completed", "error":{"message":"fixture-private"}, "output":[]}),
        json!({"status":"completed", "output":[{"type":"message", "role":"assistant", "status":"completed", "content":[{"type":"refusal", "refusal":"fixture-private"}]}]}),
    ] {
        let server = MockServer::responses(value);
        let error = AiService::new()
            .unwrap()
            .translate(
                &request(),
                &config(AiProtocol::OpenaiResponses, &server.endpoint),
            )
            .await
            .unwrap_err();
        assert!(!error.contains("fixture-private"));
        assert!(!error.contains(&server.endpoint));
        server.captured();
    }
}
