use serde_json::{json, Value};

pub struct OpenAiClient {
    http: reqwest::Client,
}

impl OpenAiClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }

    pub async fn chat_json(
        &self,
        base_url: &str,
        model: &str,
        api_key: &str,
        system: &str,
        user: Value,
    ) -> Result<Value, crate::errors::AppError> {
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
        let body = json!({
            "model": model,
            "temperature": 0,
            "response_format": { "type": "json_object" },
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": serde_json::to_string(&user).map_err(|e| crate::errors::AppError::new(e.to_string()))? }
            ]
        });
        let resp = self.http.post(&url).bearer_auth(api_key).json(&body).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(crate::errors::AppError::new(format!(
                "AI API 错误 {status}: {}",
                text.chars().take(300).collect::<String>()
            )));
        }
        let json: Value = resp.json().await?;
        let content = json
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or_else(|| crate::errors::AppError::new("AI 响应缺少 content".to_string()))?;
        Ok(serde_json::from_str(content).unwrap_or_else(|_| json!({ "raw": content })))
    }
}
