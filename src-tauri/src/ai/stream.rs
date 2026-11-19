use super::protocol::{completed_response_text, nonempty_text, reject_business_error};
use crate::model::AiProtocol;
use reqwest::Response;
use serde_json::Value;
use std::time::Duration;
use tokio::time::{timeout_at, Instant};

pub(crate) const INCOMPLETE: &str = "AI 流式响应未完成，请重试";
pub(crate) const STREAM_UNSUPPORTED: &str = "AI 服务未返回流式译文，请检查模型和接口设置";
const TIMEOUT: &str = "AI 请求超时，请重试";
const INVALID: &str = "AI 流式响应格式无效";
const REFUSED: &str = "AI 服务拒绝了此次翻译请求";

/// TCP 分块可能位于汉字或 CRLF 中间。保留行字节，遇到 SSE 空行才交付完整事件。
#[derive(Default)]
struct EventReader {
    line: Vec<u8>,
    kind: String,
    data: String,
    started: bool,
    after_cr: bool,
}

impl EventReader {
    fn next(&mut self, bytes: &mut &[u8]) -> Result<Option<(String, String)>, String> {
        while !bytes.is_empty() {
            if self.after_cr {
                self.after_cr = false;
                if let Some(rest) = bytes.strip_prefix(b"\n") {
                    *bytes = rest;
                    continue;
                }
            }
            let Some(end) = bytes.iter().position(|b| matches!(b, b'\r' | b'\n')) else {
                self.line.extend_from_slice(bytes);
                *bytes = &[];
                break;
            };
            self.line.extend_from_slice(&bytes[..end]);
            self.after_cr = bytes[end] == b'\r';
            *bytes = &bytes[end + 1..];
            let mut line = std::str::from_utf8(&self.line).map_err(|_| INVALID)?;
            if !self.started {
                line = line.strip_prefix('\u{feff}').unwrap_or(line);
                self.started = true;
            }
            if line.is_empty() {
                let kind = std::mem::take(&mut self.kind);
                if self.data.pop().is_some() {
                    return Ok(Some((kind, std::mem::take(&mut self.data))));
                }
            } else {
                let (field, value) = line.split_once(':').unwrap_or((line, ""));
                let value = value.strip_prefix(' ').unwrap_or(value);
                match field {
                    "event" => self.kind = value.into(),
                    "data" => {
                        self.data.push_str(value);
                        self.data.push('\n');
                    }
                    _ => {} // 心跳注释、id 和 retry 均不触发翻译或重发。
                }
            }
            self.line.clear();
        }
        Ok(None)
    }
}

enum Update<'a> {
    Text(&'a str),
    Reasoning,
    Metadata,
    Completed(String),
}

#[derive(Default)]
struct ChatText {
    text: String,
    stopped: bool,
}

impl ChatText {
    fn accept<'a>(&mut self, value: &'a Value) -> Result<Update<'a>, String> {
        let choices = value["choices"].as_array().ok_or(INVALID)?;
        // 部分服务省略单一候选的 index。用量帧没有候选，其它候选不属于本次译文。
        for choice in choices {
            let index = match choice.get("index") {
                Some(index) => index.as_u64().ok_or(INVALID)?,
                None if choices.len() == 1 => 0,
                None => return Err(INVALID.into()),
            };
            if index == 0 {
                return self.accept_choice(choice);
            }
        }
        Ok(Update::Metadata)
    }

    fn accept_choice<'a>(&mut self, choice: &'a Value) -> Result<Update<'a>, String> {
        let delta = &choice["delta"];
        let finish = &choice["finish_reason"];
        let filters = choice["content_filter_results"].as_object();
        if finish == "content_filter"
            || filters.is_some_and(|groups| groups.values().any(|v| v["filtered"] == true))
        {
            return Err(REFUSED.into());
        }
        if !finish.is_null() && finish != "stop" {
            return Err(INCOMPLETE.into());
        }
        // 兼容接口可能在正文前后插入仅包含审核结果与位置的注释。
        if delta.is_null()
            && finish.is_null()
            && filters.is_some()
            && choice["content_filter_offsets"].is_object()
        {
            return Ok(Update::Metadata);
        }
        let stop_only = delta.is_null() && finish == "stop";
        let calls = &delta["tool_calls"];
        if !(delta.is_object() || stop_only)
            || (!delta["role"].is_null() && delta["role"] != "assistant")
            || !(calls.is_null() || calls.as_array().is_some_and(Vec::is_empty))
            || !delta["function_call"].is_null()
        {
            return Err(INVALID.into());
        }
        if delta
            .get("refusal")
            .is_some_and(|v| !v.is_null() && v.as_str() != Some(""))
        {
            return Err(REFUSED.into());
        }
        let text = match &delta["content"] {
            Value::Null => "",
            content => content.as_str().ok_or(INVALID)?,
        };
        if self.stopped && !text.is_empty() {
            return Err(INVALID.into());
        }
        self.stopped |= finish == "stop";
        if !text.is_empty() {
            self.text.push_str(text);
            Ok(Update::Text(text))
        } else if ["reasoning_content", "reasoning"]
            .iter()
            .any(|key| delta[*key].as_str().is_some_and(|s| !s.is_empty()))
        {
            Ok(Update::Reasoning)
        } else {
            Ok(Update::Metadata)
        }
    }
}

fn responses_event<'a>(kind: &str, value: &'a Value) -> Result<Update<'a>, String> {
    let object = value.as_object().ok_or(INVALID)?;
    let event = match object.get("type") {
        Some(t) => t.as_str().filter(|s| !s.trim().is_empty()).ok_or(INVALID)?,
        None if !kind.is_empty() && kind != "message" => kind,
        None => return Err(INVALID.into()),
    };
    if !kind.is_empty() && kind != "message" && kind != event {
        return Err(INVALID.into());
    }
    match event {
        "response.output_text.delta" => match value["delta"].as_str().ok_or(INVALID)? {
            "" => Ok(Update::Metadata),
            text => Ok(Update::Text(text)),
        },
        "response.reasoning_summary_text.delta" | "response.reasoning_text.delta"
            if value["delta"].as_str().is_some_and(|s| !s.is_empty()) =>
        {
            Ok(Update::Reasoning)
        }
        // 以完成事件中按消息/文本段排序的助手输出交付结果，避免再次追加增量。
        "response.completed" => completed_response_text(&value["response"]).map(Update::Completed),
        "response.refusal.delta" | "response.refusal.done" => Err(REFUSED.into()),
        "error" | "response.failed" | "response.incomplete" | "response.cancelled" => {
            Err(INCOMPLETE.into())
        }
        _ => Ok(Update::Metadata),
    }
}

pub(super) async fn read(
    mut response: Response,
    protocol: AiProtocol,
    idle: Duration,
    mut on_delta: impl FnMut(&str),
) -> Result<String, String> {
    let mut deadline = Instant::now() + idle;
    if response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(';')
                .next()
                .is_some_and(|t| t.trim().eq_ignore_ascii_case("application/json"))
        })
    {
        if let Ok(value) = timeout_at(deadline, response.json::<Value>())
            .await
            .map_err(|_| TIMEOUT)?
        {
            reject_business_error(&value)?;
        }
        return Err(STREAM_UNSUPPORTED.into());
    }
    let mut reader = EventReader::default();
    let mut chat = ChatText::default();
    loop {
        if Instant::now() >= deadline {
            return Err(TIMEOUT.into());
        }
        let chunk = timeout_at(deadline, response.chunk())
            .await
            .map_err(|_| TIMEOUT)?
            .map_err(|e| if e.is_timeout() { TIMEOUT } else { INCOMPLETE })?;
        let Some(chunk) = chunk else {
            break;
        };
        let mut bytes = chunk.as_ref();
        while let Some((kind, data)) = reader.next(&mut bytes)? {
            if kind == "error" {
                return Err("AI 服务返回流式错误，请检查配置或重试".into());
            }
            if kind == "ping" {
                continue;
            }
            if data.trim() == "[DONE]" {
                return match protocol {
                    AiProtocol::Openai => nonempty_text(&chat.text),
                    AiProtocol::OpenaiResponses => Err(INCOMPLETE.into()),
                };
            }
            let value: Value = serde_json::from_str(&data).map_err(|_| INVALID)?;
            reject_business_error(&value)?;
            let update = match protocol {
                AiProtocol::Openai => chat.accept(&value)?,
                AiProtocol::OpenaiResponses => responses_event(&kind, &value)?,
            };
            match update {
                Update::Completed(text) => return Ok(text),
                Update::Text(text) => on_delta(text),
                Update::Reasoning => {}
                Update::Metadata => continue,
            }
            // 模型正在输出正文或推理时可以持续翻译；保活心跳不会让已停滞的请求永久等待。
            deadline = Instant::now() + idle;
        }
    }
    // 有些 Chat 服务以 stop 帧和干净的 EOF 结束。只有明确的结束信号可交付历史。
    if matches!(protocol, AiProtocol::Openai)
        && chat.stopped
        && reader.line.is_empty()
        && reader.data.is_empty()
        && reader.kind.is_empty()
    {
        nonempty_text(&chat.text)
    } else {
        Err(INCOMPLETE.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse_handles_every_byte_boundary_comments_newlines_and_multiline_data() {
        for separator in ["\n", "\r", "\r\n"] {
            let wire = [
                "\u{feff}: comment",
                "id: 2",
                "event: fragment",
                "data: 你",
                "data: 🌍",
                "",
                "data: next",
                "",
                "",
            ]
            .join(separator);
            let mut reader = EventReader::default();
            let mut events = Vec::new();
            for byte in wire.as_bytes().chunks(1) {
                let mut input = byte;
                while let Some(event) = reader.next(&mut input).unwrap() {
                    events.push(event);
                }
            }
            assert_eq!(
                events,
                [
                    ("fragment".into(), "你\n🌍".into()),
                    ("".into(), "next".into())
                ]
            );
        }
        assert!(EventReader::default()
            .next(&mut &b"data: \xff\n\n"[..])
            .is_err());
    }
}
