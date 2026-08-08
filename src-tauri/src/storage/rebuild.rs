use crate::errors::AppError;
use crate::notes_fs::title_from_markdown;
use crate::paths::join_under;
use crate::storage::notes as notes_db;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// 扫描 notes/ 下所有 .md，重建 notes 表：补缺失行并刷新 body_index。
pub fn rebuild(conn: &Connection, root: &Path) -> Result<(usize, usize), AppError> {
    let notes_root = join_under(root, Path::new("notes")).map_err(AppError::new)?;
    let mut found = 0usize;
    let mut repaired = 0usize;
    for entry in walk(&notes_root)? {
        let stem = entry
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        if !is_valid_id(&stem) {
            continue;
        }
        found += 1;
        let body = std::fs::read_to_string(&entry)
            .map_err(|e| AppError::new(format!("读取 {} 失败: {e}", entry.display())))?;
        let exists = notes_db::get_note(conn, &stem)?.is_some();
        let created = id_to_rfc3339(&stem);
        if !exists {
            notes_db::insert_note(conn, &stem, &title_from_markdown(&body), &body, &created)?;
            repaired += 1;
        } else {
            notes_db::refresh_body_index(conn, &stem, &body)?;
        }
    }
    Ok((found, repaired))
}

fn walk(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk(&path)?);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path);
        }
    }
    Ok(out)
}

fn is_valid_id(id: &str) -> bool {
    id.len() == 19 && id.as_bytes()[8] == b'-' && id.as_bytes()[15] == b'-'
}

fn id_to_rfc3339(id: &str) -> String {
    format!(
        "{}-{}-{}T{}:{}{}:00+08:00",
        &id[0..4],
        &id[4..6],
        &id[6..8],
        &id[9..11],
        &id[11..13],
        &id[13..15]
    )
}
