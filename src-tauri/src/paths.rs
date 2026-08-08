use std::path::{Component, Path, PathBuf};

/// 把相对路径安全地拼到 root 下，拒绝绝对路径、`..`、根目录/盘符前缀。
pub fn join_under(root: &Path, rel: &Path) -> Result<PathBuf, String> {
    if rel.is_absolute() {
        return Err("relative path must not be absolute".to_string());
    }
    for c in rel.components() {
        match c {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir => return Err("relative path must not contain '..'".to_string()),
            Component::RootDir | Component::Prefix(_) => {
                return Err("invalid path component".to_string());
            }
        }
    }
    Ok(root.join(rel))
}

/// 从记录 ID 推导 .md 文件路径：notes/YYYY/MM/<id>.md
pub fn derive_note_path(root: &Path, id: &str) -> PathBuf {
    let (year, month) = id_year_month(id);
    root.join("notes")
        .join(year)
        .join(month)
        .join(format!("{id}.md"))
}

/// 从记录 ID 推导附件目录：attachments/YYYY/MM/<id>/
pub fn derive_attachment_dir(root: &Path, id: &str) -> PathBuf {
    let (year, month) = id_year_month(id);
    root.join("attachments").join(year).join(month).join(id)
}

fn id_year_month(id: &str) -> (&str, &str) {
    (&id[0..4], &id[4..6])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn join_under_rejects_absolute() {
        assert!(join_under(Path::new("C:/data"), Path::new("C:/evil.txt")).is_err());
    }

    #[test]
    fn join_under_rejects_parent_dir() {
        assert!(join_under(Path::new("C:/data"), Path::new("../evil.txt")).is_err());
    }

    #[test]
    fn join_under_allows_normal() {
        assert_eq!(
            join_under(Path::new("C:/data"), Path::new("2026/08/a.md")).unwrap(),
            Path::new("C:/data/2026/08/a.md")
        );
    }

    #[test]
    fn derive_note_path_formats_correctly() {
        let id = "20260808-153012-ab12";
        let p = derive_note_path(Path::new("D:/notes"), id);
        assert_eq!(p, Path::new("D:/notes/notes/2026/08/20260808-153012-ab12.md"));
    }

    #[test]
    fn derive_attachment_dir_formats_correctly() {
        let id = "20260808-153012-ab12";
        let p = derive_attachment_dir(Path::new("D:/notes"), id);
        assert_eq!(
            p,
            Path::new("D:/notes/attachments/2026/08/20260808-153012-ab12")
        );
    }
}
