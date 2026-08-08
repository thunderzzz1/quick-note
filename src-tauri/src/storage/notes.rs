use rusqlite::{params, Connection};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct NoteMeta {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub ai_status: String,
    pub category_id: Option<i64>,
    pub summary: Option<String>,
    pub keywords: Option<String>,
    pub tags: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NoteWithBody {
    #[serde(flatten)]
    pub meta: NoteMeta,
    pub body_index: String,
}

pub fn insert_note(
    conn: &Connection,
    id: &str,
    title: &str,
    body_index: &str,
    created_at: &str,
) -> rusqlite::Result<NoteMeta> {
    conn.execute(
        "INSERT INTO notes (id, title, created_at, updated_at, ai_status, body_index)
         VALUES (?1, ?2, ?3, ?3, 'pending', ?4)",
        params![id, title, created_at, body_index],
    )?;
    get_note(conn, id)?
        .map(|n| n.meta)
        .ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn get_note(conn: &Connection, id: &str) -> rusqlite::Result<Option<NoteWithBody>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, created_at, updated_at, ai_status, category_id,
                summary, keywords, tags, body_index FROM notes WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], row_to_note_with_body)?;
    rows.next().transpose()
}

pub fn list_notes_by_date(conn: &Connection, date: &str) -> rusqlite::Result<Vec<NoteMeta>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, created_at, updated_at, ai_status, category_id,
                summary, keywords, tags FROM notes WHERE substr(created_at, 1, 10) = ?1
         ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map(params![date], row_to_note_meta)?;
    rows.collect()
}

pub fn list_notes_by_category(
    conn: &Connection,
    category_id: i64,
) -> rusqlite::Result<Vec<NoteMeta>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, created_at, updated_at, ai_status, category_id,
                summary, keywords, tags FROM notes WHERE category_id = ?1
         ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map(params![category_id], row_to_note_meta)?;
    rows.collect()
}

pub fn list_all(conn: &Connection) -> rusqlite::Result<Vec<NoteMeta>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, created_at, updated_at, ai_status, category_id,
                summary, keywords, tags FROM notes ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], row_to_note_meta)?;
    rows.collect()
}

pub fn search(conn: &Connection, q: &str) -> rusqlite::Result<Vec<NoteMeta>> {
    let like = format!("%{}%", q);
    let mut stmt = conn.prepare(
        "SELECT id, title, created_at, updated_at, ai_status, category_id,
                summary, keywords, tags FROM notes
         WHERE title LIKE ?1 OR body_index LIKE ?1 ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map(params![like], row_to_note_meta)?;
    rows.collect()
}

pub fn update_ai_meta(
    conn: &Connection,
    id: &str,
    category_id: Option<i64>,
    summary: Option<&str>,
    keywords: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE notes SET category_id = ?1, summary = ?2, keywords = ?3,
         ai_status = 'confirmed', updated_at = ?4 WHERE id = ?5",
        params![
            category_id,
            summary,
            keywords,
            chrono::Utc::now().to_rfc3339(),
            id
        ],
    )?;
    Ok(())
}

pub fn set_ai_status(conn: &Connection, id: &str, status: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE notes SET ai_status = ?1, updated_at = ?2 WHERE id = ?3",
        params![status, chrono::Utc::now().to_rfc3339(), id],
    )?;
    Ok(())
}

pub fn pending_ids(conn: &Connection, date: &str) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT id FROM notes WHERE substr(created_at, 1, 10) = ?1 AND ai_status = 'pending'
         ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(params![date], |r| r.get::<_, String>(0))?;
    rows.collect()
}

pub fn refresh_body_index(conn: &Connection, id: &str, body: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE notes SET body_index = ?1, updated_at = ?2 WHERE id = ?3",
        params![body, chrono::Utc::now().to_rfc3339(), id],
    )?;
    Ok(())
}

fn row_to_note_meta(r: &rusqlite::Row) -> rusqlite::Result<NoteMeta> {
    Ok(NoteMeta {
        id: r.get(0)?,
        title: r.get(1)?,
        created_at: r.get(2)?,
        updated_at: r.get(3)?,
        ai_status: r.get(4)?,
        category_id: r.get(5)?,
        summary: r.get(6)?,
        keywords: r.get(7)?,
        tags: r.get(8)?,
    })
}

fn row_to_note_with_body(r: &rusqlite::Row) -> rusqlite::Result<NoteWithBody> {
    let meta = NoteMeta {
        id: r.get(0)?,
        title: r.get(1)?,
        created_at: r.get(2)?,
        updated_at: r.get(3)?,
        ai_status: r.get(4)?,
        category_id: r.get(5)?,
        summary: r.get(6)?,
        keywords: r.get(7)?,
        tags: r.get(8)?,
    };
    Ok(NoteWithBody {
        meta,
        body_index: r.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use crate::storage::{open_in_memory, Storage};

    fn storage() -> Storage {
        let conn = open_in_memory().unwrap();
        Storage::new(conn)
    }

    #[test]
    fn insert_and_get_note() {
        let s = storage();
        let meta = s
            .insert_note(
                "20260808-153012-ab12",
                "周报",
                "明天交周报",
                "2026-08-08T15:30:12+08:00",
            )
            .unwrap();
        assert_eq!(meta.id, "20260808-153012-ab12");
        assert_eq!(meta.title, "周报");
        let got = s.get_note("20260808-153012-ab12").unwrap().unwrap();
        assert_eq!(got.body_index, "明天交周报");
    }

    #[test]
    fn update_note_metadata_keeps_body_index() {
        let s = storage();
        s.insert_note(
            "20260808-153012-ab12",
            "周报",
            "明天交周报",
            "2026-08-08T15:30:12+08:00",
        )
        .unwrap();
        s.update_ai_meta(
            "20260808-153012-ab12",
            Some(1),
            Some("明天下午3点交周报"),
            Some(r#"["周报"]"#),
        )
        .unwrap();
        let got = s.get_note("20260808-153012-ab12").unwrap().unwrap();
        assert_eq!(got.meta.ai_status, "confirmed");
        assert_eq!(got.meta.summary.as_deref(), Some("明天下午3点交周报"));
        assert_eq!(got.body_index, "明天交周报");
    }

    #[test]
    fn list_notes_filters_by_date() {
        let s = storage();
        s.insert_note(
            "20260808-153012-ab12",
            "a",
            "x",
            "2026-08-08T15:30:12+08:00",
        )
        .unwrap();
        s.insert_note(
            "20260809-100000-cd34",
            "b",
            "y",
            "2026-08-09T10:00:00+08:00",
        )
        .unwrap();
        let today = s.list_notes_by_date("2026-08-08").unwrap();
        assert_eq!(today.len(), 1);
        assert_eq!(today[0].id, "20260808-153012-ab12");
    }
}
