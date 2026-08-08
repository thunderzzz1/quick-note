use rusqlite::Connection;

pub const SCHEMA_VERSION: i64 = 1;

pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if v < 1 {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS notes (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                ai_status TEXT NOT NULL DEFAULT 'pending',
                category_id INTEGER,
                summary TEXT,
                keywords TEXT,
                tags TEXT,
                body_index TEXT NOT NULL DEFAULT ''
            );
            CREATE TABLE IF NOT EXISTS categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                origin TEXT NOT NULL DEFAULT 'user',
                enabled INTEGER NOT NULL DEFAULT 1,
                sort_order INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS suggestions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                note_id TEXT NOT NULL REFERENCES notes(id),
                ai_category TEXT,
                new_category_proposal TEXT,
                summary TEXT,
                keywords TEXT,
                raw_response TEXT,
                status TEXT NOT NULL DEFAULT 'suggested',
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS daily_summaries (
                date TEXT PRIMARY KEY,
                summary TEXT NOT NULL
            );
            INSERT OR IGNORE INTO categories (name, origin, enabled, sort_order, created_at) VALUES
                ('待办', 'builtin', 1, 1, '2026-01-01T00:00:00+08:00'),
                ('进度', 'builtin', 1, 2, '2026-01-01T00:00:00+08:00'),
                ('提醒', 'builtin', 1, 3, '2026-01-01T00:00:00+08:00'),
                ('知识库', 'builtin', 1, 4, '2026-01-01T00:00:00+08:00'),
                ('其他', 'builtin', 1, 999, '2026-01-01T00:00:00+08:00');
            "#,
        )?;
        conn.pragma_update(None, "user_version", 1)?;
    }
    Ok(())
}
