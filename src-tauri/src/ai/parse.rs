use serde::Deserialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Deserialize)]
pub struct NoteSuggestion {
    pub note_id: String,
    pub category: Option<String>,
    #[serde(default)]
    pub new_category_proposal: Option<String>,
    pub summary: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrganizationResponse {
    pub notes: Vec<NoteSuggestion>,
    #[serde(default)]
    pub daily_summary: Option<String>,
}

pub fn parse_organization(raw: &str) -> Result<Vec<NoteSuggestion>, String> {
    let resp: OrganizationResponse =
        serde_json::from_str(raw).map_err(|e| format!("AI 返回不是合法 JSON: {e}"))?;
    if resp.notes.is_empty() {
        return Err("AI 未返回任何记录".into());
    }
    Ok(resp.notes)
}

pub fn validate_against_batch(
    suggestions: &[NoteSuggestion],
    batch_ids: &[String],
) -> Result<(), String> {
    let allowed: HashSet<&str> = batch_ids.iter().map(String::as_str).collect();
    for s in suggestions {
        if !allowed.contains(s.note_id.as_str()) {
            return Err(format!("AI 返回了未知记录 ID: {}", s.note_id));
        }
    }
    Ok(())
}

pub fn parse_daily_summary(raw: &str) -> Option<String> {
    serde_json::from_str::<OrganizationResponse>(raw)
        .ok()?
        .daily_summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_response() {
        let raw = r#"{
            "notes": [
                {"note_id": "20260808-153012-ab12", "category": "待办",
                 "new_category_proposal": null, "summary": "明天交周报", "keywords": ["周报"]}
            ],
            "daily_summary": "今天主要是周报"
        }"#;
        let parsed = parse_organization(raw).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].note_id, "20260808-153012-ab12");
        assert_eq!(parsed[0].category.as_deref(), Some("待办"));
    }

    #[test]
    fn rejects_unknown_note_id() {
        let raw = r#"{"notes": [{"note_id": "nope", "category": "待办"}], "daily_summary": ""}"#;
        let parsed = parse_organization(raw).unwrap();
        let batch = vec!["20260808-153012-ab12".to_string()];
        assert!(validate_against_batch(&parsed, &batch).is_err());
    }

    #[test]
    fn rejects_missing_notes_field() {
        assert!(parse_organization("{}").is_err());
    }
}
