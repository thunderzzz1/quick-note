use rusqlite::{params, Connection};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Suggestion {
    pub id: i64,
    pub note_id: String,
    pub ai_category: Option<String>,
    pub new_category_proposal: Option<String>,
    pub summary: Option<String>,
    pub keywords: Option<String>,
    pub status: String,
    pub created_at: String,
}

pub fn insert_batch(
    conn: &Connection,
    rows: &[(
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
    )],
) -> rusqlite::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut stmt = conn.prepare(
        "INSERT INTO suggestions (note_id, ai_category, new_category_proposal, summary, keywords, raw_response, status, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'suggested', ?7)",
    )?;
    for (note_id, cat, proposal, summary, keywords, raw) in rows {
        stmt.execute(params![note_id, cat, proposal, summary, keywords, raw, now])?;
    }
    Ok(())
}

pub fn list_by_date(conn: &Connection, date: &str) -> rusqlite::Result<Vec<Suggestion>> {
    let mut stmt = conn.prepare(
        "SELECT id, note_id, ai_category, new_category_proposal, summary, keywords, status, created_at
         FROM suggestions WHERE substr(created_at, 1, 10) = ?1 ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(params![date], row_to_suggestion)?;
    rows.collect()
}

pub fn get(conn: &Connection, id: i64) -> rusqlite::Result<Option<Suggestion>> {
    let mut stmt = conn.prepare(
        "SELECT id, note_id, ai_category, new_category_proposal, summary, keywords, status, created_at
         FROM suggestions WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], row_to_suggestion)?;
    rows.next().transpose()
}

pub fn set_status(conn: &Connection, id: i64, status: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE suggestions SET status = ?1 WHERE id = ?2",
        params![status, id],
    )?;
    Ok(())
}

pub fn upsert_daily_summary(conn: &Connection, date: &str, summary: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO daily_summaries (date, summary) VALUES (?1, ?2)
         ON CONFLICT(date) DO UPDATE SET summary = excluded.summary",
        params![date, summary],
    )?;
    Ok(())
}

fn row_to_suggestion(r: &rusqlite::Row) -> rusqlite::Result<Suggestion> {
    Ok(Suggestion {
        id: r.get(0)?,
        note_id: r.get(1)?,
        ai_category: r.get(2)?,
        new_category_proposal: r.get(3)?,
        summary: r.get(4)?,
        keywords: r.get(5)?,
        status: r.get(6)?,
        created_at: r.get(7)?,
    })
}
