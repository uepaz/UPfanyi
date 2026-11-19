use crate::model::{AiConfig, AiProtocol, TranslationRequest};
use serde_json::{json, Value};

pub(super) fn request_body(request: &TranslationRequest, config: &AiConfig) -> Value {
    let prompt = config
        .system_prompt
        .replace("{source_lang}", &language_name(&request.source_lang))
        .replace("{target_lang}", &language_name(&request.target_lang));
    let messages = json!([
        {"role": "system", "content": prompt},
        {"role": "user", "content": request.text},
    ]);
    let model = config.model.trim();
    match config.protocol {
        AiProtocol::Openai => json!({
            "model": model,
            "messages": messages,
            "stream": false,
            "temperature": 0.3,
            "max_tokens": 4096,
        }),
        AiProtocol::OpenaiResponses => json!({
            "model": model,
            "instructions": prompt,
            "input": request.text,
            "stream": false,
            "store": false,
        }),
    }
}

pub(super) fn response_text(protocol: AiProtocol, value: &Value) -> Result<String, String> {
    reject_business_error(value)?;
    let path = match protocol {
        AiProtocol::Openai => "/choices/0/message/content",
        AiProtocol::OpenaiResponses => return responses_text(value),
    };
    nonempty_text(
        value
            .pointer(path)
            .and_then(Value::as_str)
            .ok_or("AI 响应缺少译文或译文格式无效")?,
    )
}

pub(super) fn reject_business_error(value: &Value) -> Result<(), String> {
    if value.get("error").is_some_and(|error| !error.is_null()) {
        // 兼容服务的 error/code/type 也可能回显秘密，不将其当成安全错误码。
        return Err("AI 服务返回业务错误，请检查配置、权限或额度".into());
    }
    Ok(())
}

fn responses_text(value: &Value) -> Result<String, String> {
    if value.get("status").and_then(Value::as_str) != Some("completed") {
        return Err("AI Responses 响应未完成或状态无效，请重试".into());
    }
    let output = value
        .get("output")
        .and_then(Value::as_array)
        .ok_or("AI Responses 输出格式无效")?;
    let mut text = String::new();
    for item in output {
        let item_type = item
            .get("type")
            .and_then(Value::as_str)
            .ok_or("AI Responses 输出项格式无效")?;
        // 推理和工具项可能包含文本，但它们不是可交付的助手译文。
        if item_type != "message" {
            continue;
        }
        if item.get("role").and_then(Value::as_str) != Some("assistant")
            || item.get("status").and_then(Value::as_str) != Some("completed")
        {
            return Err("AI Responses 助手消息未完成或格式无效".into());
        }
        let content = item
            .get("content")
            .and_then(Value::as_array)
            .ok_or("AI Responses 助手消息格式无效")?;
        for part in content {
            match part.get("type").and_then(Value::as_str) {
                Some("output_text") => text.push_str(
                    part.get("text")
                        .and_then(Value::as_str)
                        .ok_or("AI Responses 译文格式无效")?,
                ),
                Some("refusal") => return Err("AI 服务拒绝了此次翻译请求".into()),
                _ => return Err("AI Responses 文本内容格式无效".into()),
            }
        }
    }
    nonempty_text(&text)
}

fn nonempty_text(text: &str) -> Result<String, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("AI 服务返回空译文".into());
    }
    Ok(text.into())
}

fn language_name(language: &str) -> String {
    match language.to_ascii_lowercase().as_str() {
        "auto" => "自动检测语言".into(),
        "zh" => "中文".into(),
        "zh-tw" => "繁体中文".into(),
        "en" => "英语".into(),
        "ja" => "日语".into(),
        "ko" => "韩语".into(),
        "fr" => "法语".into(),
        "de" => "德语".into(),
        "es" => "西班牙语".into(),
        "ru" => "俄语".into(),
        // 未列出的合法代码直接进入提示词，不转换为某个厂商专用语言码。
        _ => language.into(),
    }
}
