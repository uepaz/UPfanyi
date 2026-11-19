use super::protocol::reject_business_error;
use serde_json::Value;
use std::collections::HashSet;

pub(super) fn model_names(value: &Value) -> Result<Vec<String>, String> {
    reject_business_error(value)?;
    let models = value
        .get("data")
        .and_then(Value::as_array)
        .ok_or("AI 服务返回的模型列表格式无效")?;
    let mut names = Vec::new();
    let mut seen = HashSet::new();
    for model in models {
        let name = model
            .get("id")
            .and_then(Value::as_str)
            .ok_or("AI 服务返回的模型名称格式无效")?
            .trim();
        if !name.is_empty() && seen.insert(name.to_owned()) {
            names.push(name.to_owned());
        }
    }
    Ok(names)
}
