//! AI 翻译服务：只使用传入配置快照，不读取数据库或写入历史。

mod discovery;
mod endpoint;
mod protocol;
mod stream;
pub(crate) use stream::{INCOMPLETE, STREAM_UNSUPPORTED};

#[cfg(test)]
mod discovery_tests;
#[cfg(test)]
mod responses_tests;
#[cfg(test)]
mod stream_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
use crate::model::AiProtocol;
use crate::{
    model::{AiConfig, Translation, TranslationRequest},
    validation::validate_languages,
};
use chrono::Utc;
use endpoint::{models_endpoint, translation_endpoint, validate_endpoint};
use reqwest::{Client, RequestBuilder, Response};
use serde_json::Value;
use std::time::Duration;
use url::Url;

const READ_TIMEOUT: Duration = Duration::from_secs(60);

pub struct AiService {
    client: Client,
}

impl AiService {
    pub fn new() -> Result<Self, String> {
        let client = Client::builder()
            // 自定义端点可携带凭据和原文，禁止重定向到另一服务。
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(30))
            // 流式译文可能生成很久；每次读到数据后重新计算等待时间。
            .read_timeout(READ_TIMEOUT)
            .build()
            .map_err(|_| "无法初始化 AI 网络客户端".to_owned())?;
        Ok(Self { client })
    }

    pub async fn translate(
        &self,
        request: &TranslationRequest,
        config: &AiConfig,
    ) -> Result<Translation, String> {
        self.translate_stream(request, config, |_| {}).await
    }

    pub(crate) async fn translate_stream(
        &self,
        request: &TranslationRequest,
        config: &AiConfig,
        on_delta: impl FnMut(&str) + Send,
    ) -> Result<Translation, String> {
        if request.text.trim().is_empty() {
            return Err("请输入需要翻译的文本".into());
        }
        validate_languages(&request.source_lang, &request.target_lang)?;
        ensure_configured(config)?;

        let endpoint = translation_endpoint(config.protocol, validate_endpoint(&config.endpoint)?);
        let mut builder = self
            .client
            .post(endpoint)
            .json(&protocol::request_body(request, config));
        builder = with_auth(builder, config);
        let response = send(builder.header(reqwest::header::ACCEPT, "text/event-stream")).await?;
        let translated_text =
            stream::read(response, config.protocol, READ_TIMEOUT, on_delta).await?;
        Ok(Translation {
            source_text: request.text.clone(),
            translated_text,
            source_lang: request.source_lang.clone(),
            target_lang: request.target_lang.clone(),
            model: config.model.trim().into(),
            create_time: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            warning: None,
        })
    }

    pub async fn list_models(&self, config: &AiConfig) -> Result<Vec<String>, String> {
        let endpoint = validate_endpoint(&config.endpoint)?;
        validate_auth(config, &endpoint)?;
        // 列表只依赖服务地址与凭据，不要求已选模型，也不发送提示词或翻译。
        let endpoint = models_endpoint(endpoint)?;
        let value = send_json(with_auth(self.client.get(endpoint), config)).await?;
        discovery::model_names(&value)
    }

    pub async fn test(&self, config: &AiConfig) -> Result<Translation, String> {
        // 连接测试使用同一协议和校验路径，但只接收独立草稿，不改变正式状态。
        self.translate(
            &TranslationRequest {
                text: "Hello".into(),
                source_lang: "en".into(),
                target_lang: "zh".into(),
            },
            config,
        )
        .await
    }
}

fn with_auth(builder: RequestBuilder, config: &AiConfig) -> RequestBuilder {
    if !config.api_key.trim().is_empty() {
        builder.bearer_auth(config.api_key.trim())
    } else {
        builder
    }
}

async fn send(builder: RequestBuilder) -> Result<Response, String> {
    let response = builder.send().await.map_err(|error| {
        // reqwest 错误可能包含完整 URL；翻译与模型发现共用脱敏边界。
        if error.is_timeout() {
            "AI 请求超时，请检查网络或稍后重试".to_owned()
        } else if error.is_connect() {
            "无法连接 AI 服务，请检查网络和端点".to_owned()
        } else {
            "AI 请求失败，请检查网络或配置".to_owned()
        }
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "AI 服务返回 HTTP {}，请检查服务状态、密钥或额度",
            status.as_u16()
        ));
    }
    Ok(response)
}

async fn send_json(builder: RequestBuilder) -> Result<Value, String> {
    let bytes = send(builder.timeout(READ_TIMEOUT))
        .await?
        .bytes()
        .await
        .map_err(|_| "无法读取 AI 响应".to_owned())?;
    serde_json::from_slice(&bytes).map_err(|_| "AI 服务返回无效 JSON".to_owned())
}

/// 保存允许未配齐的草稿；非空服务地址必须是安全的域名或 HTTP(S) URL。
pub(crate) fn validate_config(config: &AiConfig) -> Result<(), String> {
    if !config.endpoint.trim().is_empty() {
        validate_endpoint(&config.endpoint)?;
    }
    Ok(())
}

pub(crate) fn ensure_configured(config: &AiConfig) -> Result<(), String> {
    if config.endpoint.trim().is_empty() {
        return Err("请填写 AI 服务地址或域名".into());
    }
    let endpoint = validate_endpoint(&config.endpoint)?;
    if config.model.trim().is_empty() {
        return Err("请填写 AI 模型名称".into());
    }
    validate_auth(config, &endpoint)
}

fn validate_auth(config: &AiConfig, endpoint: &Url) -> Result<(), String> {
    if endpoint.host_str() == Some("api.openai.com") && config.api_key.trim().is_empty() {
        return Err("官方 OpenAI 服务需要 API Key".into());
    }
    Ok(())
}
