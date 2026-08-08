use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// 原子写入：先写同目录临时文件并 fsync，再 rename 覆盖目标。
pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "path has no parent")
    })?;
    fs::create_dir_all(parent)?;

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let tmp = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
        stamp
    ));

    let result = (|| {
        let mut f = File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        fs::rename(&tmp, path)?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn atomic_write_creates_and_replaces() {
        let dir = temp_dir().join(format!("qnatomtest-{}", std::process::id()));
        let file = dir.join("a.md");
        atomic_write(&file, b"hello").unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "hello");
        atomic_write(&file, b"world").unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "world");
        fs::remove_dir_all(dir).unwrap();
    }
}
