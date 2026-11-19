use crate::model::AiProtocol;
use url::Url;

pub(super) fn validate_endpoint(address: &str) -> Result<Url, String> {
    let address = address.trim();
    let invalid = "AI 服务地址必须是 HTTP(S) URL 或有效域名";
    if address.is_empty()
        || address.contains('\\')
        || address
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err(invalid.into());
    }
    let explicit_scheme = address.split_once("://");
    let normalized;
    let text = if let Some((scheme, authority)) = explicit_scheme {
        if !(scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
            || authority.starts_with('/')
        {
            return Err(invalid.into());
        }
        address
    } else {
        // 裸域名只补 HTTPS，不修补拼错的协议，也不静默切换服务。
        let lower = address.to_ascii_lowercase();
        if address.starts_with('/') || lower.starts_with("http:") || lower.starts_with("https:") {
            return Err(invalid.into());
        }
        normalized = format!("https://{address}");
        &normalized
    };
    let url = Url::parse(text).map_err(|_| invalid.to_owned())?;
    let host = url.host_str().ok_or_else(|| invalid.to_owned())?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err("AI 服务地址必须无内嵌账号和片段".into());
    }
    if explicit_scheme.is_none()
        && host != "localhost"
        && !host.contains('.')
        && !matches!(url.host(), Some(url::Host::Ipv4(_) | url::Host::Ipv6(_)))
    {
        return Err(invalid.into());
    }
    Ok(url)
}

pub(super) fn translation_endpoint(protocol: AiProtocol, mut address: Url) -> Url {
    if let Some(base) = api_base_path(address.path()) {
        let suffix = match protocol {
            AiProtocol::Openai => "chat/completions",
            AiProtocol::OpenaiResponses => "responses",
        };
        address.set_path(&format!("{base}/{suffix}"));
    }
    // 特殊完整翻译地址仍按原值发送；列表路径不能从这种地址猜测。
    address
}

pub(super) fn models_endpoint(mut address: Url) -> Result<Url, String> {
    let base = api_base_path(address.path())
        .ok_or("此特殊地址无法确定模型列表路径，请填写服务域名或基础地址")?;
    address.set_path(&format!("{base}/models"));
    Ok(address)
}

fn api_base_path(path: &str) -> Option<&str> {
    let path = path.trim_end_matches('/');
    // 根域名与显式版本基础路径共用生成规则，保留代理前缀、端口和 query。
    if path.is_empty() {
        Some("/v1")
    } else if path.ends_with("/v1") {
        Some(path)
    } else {
        path.strip_suffix("/chat/completions")
            .or_else(|| path.strip_suffix("/responses"))
    }
}
