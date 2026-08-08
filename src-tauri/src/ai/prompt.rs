use serde_json::json;

pub fn build_system_prompt(categories: &[String], max_categories: usize) -> String {
    format!(
        "你是笔记整理助手。规则：\n\
         1. 分类只从给定列表选择，优先复用；\n\
         2. 只有内容明显不属于任何现有分类时，才提议一个新分类（new_category_proposal），且新分类总数不能使有效分类超过 {max_categories} 个；\n\
         3. 超过上限或无法判断的内容归入「其他」；\n\
         4. 必须只输出 JSON，不要输出任何解释。\n\
         可选分类：{}",
        categories.join("、")
    )
}

pub fn build_user_payload(
    notes: &[(String, String)],
    daily_summary_hint: &str,
) -> serde_json::Value {
    let notes_json: Vec<serde_json::Value> = notes
        .iter()
        .map(|(id, text)| json!({ "note_id": id, "content": text }))
        .collect();
    json!({
        "notes": notes_json,
        "output_schema_hint": {
            "notes": [{
                "note_id": "记录 ID 原样返回",
                "category": "分类名",
                "new_category_proposal": null,
                "summary": "一句话摘要",
                "keywords": ["关键词"]
            }],
            "daily_summary": "今日概览一句话"
        },
        "note": daily_summary_hint
    })
}
