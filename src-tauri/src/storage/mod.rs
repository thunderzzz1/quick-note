pub mod categories;
pub mod db;
pub mod notes;
pub mod rebuild;
pub mod suggestions;

use rusqlite::Connection;

pub struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    // ---- notes 门面方法（委托 notes.rs 自由函数）----
    pub fn insert_note(
        &self,
        id: &str,
        title: &str,
        body: &str,
        created_at: &str,
    ) -> rusqlite::Result<notes::NoteMeta> {
        notes::insert_note(&self.conn, id, title, body, created_at)
    }

    pub fn get_note(&self, id: &str) -> rusqlite::Result<Option<notes::NoteWithBody>> {
        notes::get_note(&self.conn, id)
    }

    pub fn list_notes_by_date(&self, date: &str) -> rusqlite::Result<Vec<notes::NoteMeta>> {
        notes::list_notes_by_date(&self.conn, date)
    }

    pub fn update_ai_meta(
        &self,
        id: &str,
        category_id: Option<i64>,
        summary: Option<&str>,
        keywords: Option<&str>,
    ) -> rusqlite::Result<()> {
        notes::update_ai_meta(&self.conn, id, category_id, summary, keywords)
    }

    pub fn set_ai_status(&self, id: &str, status: &str) -> rusqlite::Result<()> {
        notes::set_ai_status(&self.conn, id, status)
    }

    pub fn pending_ids(&self, date: &str) -> rusqlite::Result<Vec<String>> {
        notes::pending_ids(&self.conn, date)
    }

    pub fn open(path: &std::path::Path) -> crate::errors::AppResult<Self> {
        let conn = Connection::open(path)?;
        db::migrate(&conn)?;
        Ok(Self::new(conn))
    }
}

#[cfg(test)]
pub fn open_in_memory() -> rusqlite::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    db::migrate(&conn)?;
    Ok(conn)
}
