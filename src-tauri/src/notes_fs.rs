use crate::io_atomic::atomic_write;
use crate::paths::{derive_note_path, join_under};
use std::path::Path;

pub fn generate_id(now: &chrono::DateTime<chrono::FixedOffset>) -> String {
    use rand::Rng;
    let suffix: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(4)
        .map(char::from)
        .collect::<String>()
        .to_ascii_lowercase();
    format!("{}-{suffix}", now.format("%Y%m%d-%H%M%S"))
}

pub fn title_from_markdown(md: &str) -> String {
    md.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(|l| l.trim_start_matches('#').trim().chars().take(60).collect())
        .unwrap_or_else(|| "无标题".to_string())
}

pub fn save_note_file(root: &Path, id: &str, markdown: &str) -> Result<(), String> {
    let path = derive_note_path(root, id);
    atomic_write(&path, markdown.as_bytes()).map_err(|e| format!("保存笔记失败: {e}"))
}

pub fn read_note_file(root: &Path, id: &str) -> Result<String, String> {
    let path = derive_note_path(root, id);
    std::fs::read_to_string(&path).map_err(|e| format!("读取笔记失败: {e}"))
}

fn extension_for_mime(mime: &str) -> Option<&'static str> {
    match mime.trim().to_ascii_lowercase().as_str() {
        "image/png" => Some("png"),
        "image/jpeg" => Some("jpg"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        "image/bmp" => Some("bmp"),
        _ => None,
    }
}

/// 原始文件名只用于校验，实际落盘统一命名为 img_NNN.<ext>。
fn validate_original_filename(name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("图片文件名为空".into());
    }
    if trimmed == "." || trimmed == ".." || trimmed.starts_with('.') {
        return Err("图片文件名不合法".into());
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains(':') {
        return Err("图片文件名不能包含路径分隔符".into());
    }
    Ok(())
}

/// 原始文件名带扩展名时，必须与 MIME 推导的扩展名一致（防伪装文件）。
fn extension_matches_mime(original: &str, mime_ext: &str) -> bool {
    let Some(ext) = Path::new(original)
        .extension()
        .and_then(|e| e.to_str())
    else {
        return true; // 无扩展名时以 MIME 为准
    };
    let ext = ext.to_ascii_lowercase();
    if mime_ext == "jpg" {
        ext == "jpg" || ext == "jpeg"
    } else {
        ext == mime_ext
    }
}

pub fn save_pasted_image(
    root: &Path,
    note_id: &str,
    original_filename: &str,
    bytes: &[u8],
    mime: &str,
) -> Result<String, String> {
    validate_original_filename(original_filename)?;
    let ext = extension_for_mime(mime).ok_or_else(|| format!("不支持的图片类型: {mime}"))?;
    if !extension_matches_mime(original_filename, ext) {
        return Err("图片扩展名与文件类型不一致".into());
    }
    let (year, month) = (&note_id[0..4], &note_id[4..6]);

    let mut index = 1usize;
    loop {
        let name = format!("img_{index:03}.{ext}");
        let rel = format!("attachments/{year}/{month}/{note_id}/{name}");
        let abs = join_under(root, Path::new(&rel)).map_err(|e| e)?;
        if !abs.exists() {
            atomic_write(&abs, bytes).map_err(|e| format!("保存图片失败: {e}"))?;
            return Ok(rel);
        }
        index += 1;
        if index > 999 {
            return Err("图片数量过多".into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::derive_note_path;
    use std::env::temp_dir;

    #[test]
    fn save_note_file_writes_markdown() {
        let root = temp_dir().join(format!("qnfs-{}", std::process::id()));
        let id = "20260808-153012-ab12";
        save_note_file(&root, id, "# 周报\n明天交").unwrap();
        let p = derive_note_path(&root, id);
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "# 周报\n明天交");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn save_pasted_image_rejects_bad_filename() {
        let root = temp_dir().join(format!("qnfs2-{}", std::process::id()));
        let id = "20260808-153012-ab12";
        assert!(save_pasted_image(&root, id, "../../evil.png", b"x", "image/png").is_err());
        assert!(save_pasted_image(&root, id, "evil.exe", b"x", "image/png").is_err());
        assert!(save_pasted_image(&root, id, ".hidden.png", b"x", "image/png").is_err());
    }

    #[test]
    fn save_pasted_image_writes_under_attachment_dir() {
        let root = temp_dir().join(format!("qnfs3-{}", std::process::id()));
        let id = "20260808-153012-ab12";
        let rel = save_pasted_image(&root, id, "截图.png", b"pngbytes", "image/png").unwrap();
        assert_eq!(rel, format!("attachments/2026/08/{id}/img_001.png"));
        assert!(root.join(&rel).exists());
        std::fs::remove_dir_all(&root).ok();
    }
}
