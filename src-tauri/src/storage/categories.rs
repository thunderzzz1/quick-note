use rusqlite::{params, Connection};
use serde::Serialize;

pub const MAX_ACTIVE_CATEGORIES: i64 = 10;

#[derive(Debug, Clone, Serialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub origin: String,
    pub enabled: bool,
    pub sort_order: i64,
    pub created_at: String,
}

pub fn list(conn: &Connection) -> rusqlite::Result<Vec<Category>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, origin, enabled, sort_order, created_at FROM categories
         ORDER BY sort_order ASC, id ASC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(Category {
            id: r.get(0)?,
            name: r.get(1)?,
            origin: r.get(2)?,
            enabled: r.get::<_, i64>(3)? != 0,
            sort_order: r.get(4)?,
            created_at: r.get(5)?,
        })
    })?;
    rows.collect()
}

pub fn id_by_name(conn: &Connection, name: &str) -> rusqlite::Result<Option<i64>> {
    let mut stmt = conn.prepare("SELECT id FROM categories WHERE name = ?1")?;
    let mut rows = stmt.query_map(params![name], |r| r.get::<_, i64>(0))?;
    rows.next().transpose()
}

pub fn fallback_id(conn: &Connection) -> rusqlite::Result<i64> {
    id_by_name(conn, "其他")?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn active_count(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM categories WHERE enabled = 1 AND name != '其他'",
        [],
        |r| r.get(0),
    )
}

pub fn create(conn: &Connection, name: &str, origin: &str) -> rusqlite::Result<Category> {
    if active_count(conn)? >= MAX_ACTIVE_CATEGORIES {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    conn.execute(
        "INSERT INTO categories (name, origin, enabled, sort_order, created_at)
         VALUES (?1, ?2, 1, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM categories), ?3)",
        params![name, origin, chrono::Utc::now().to_rfc3339()],
    )?;
    let id = id_by_name(conn, name)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    list(conn)?
        .into_iter()
        .find(|c| c.id == id)
        .ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn rename(conn: &Connection, id: i64, new_name: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE categories SET name = ?1 WHERE id = ?2",
        params![new_name, id],
    )?;
    Ok(())
}

pub fn set_enabled(conn: &Connection, id: i64, enabled: bool) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE categories SET enabled = ?1 WHERE id = ?2",
        params![if enabled { 1 } else { 0 }, id],
    )?;
    Ok(())
}

pub fn delete_and_merge(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    let fallback = fallback_id(conn)?;
    conn.execute(
        "UPDATE notes SET category_id = ?1 WHERE category_id = ?2",
        params![fallback, id],
    )?;
    conn.execute("DELETE FROM categories WHERE id = ?1", params![id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::open_in_memory;

    #[test]
    fn builtin_categories_seeded() {
        let conn = open_in_memory().unwrap();
        let cats = list(&conn).unwrap();
        assert!(cats.iter().any(|c| c.name == "待办"));
        assert!(cats.iter().any(|c| c.name == "其他"));
    }

    #[test]
    fn fallback_is_other() {
        let conn = open_in_memory().unwrap();
        let name = list(&conn)
            .unwrap()
            .into_iter()
            .find(|c| c.id == fallback_id(&conn).unwrap())
            .unwrap()
            .name;
        assert_eq!(name, "其他");
    }

    #[test]
    fn cannot_create_beyond_cap() {
        let conn = open_in_memory().unwrap();
        for i in 0..6 {
            create(&conn, &format!("新分类{i}"), "ai").unwrap();
        }
        assert!(create(&conn, "超出的分类", "ai").is_err());
    }
}
