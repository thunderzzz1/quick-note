# QuickNote MVP 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 QuickNote v0.1——一个 Windows 优先的 Tauri 2 快速记录应用：全局快捷键呼出悬浮窗、Markdown 所见即所得 + 图片粘贴、本地 Markdown/SQLite 存储、云端 AI 每晚自动整理并支持人工确认。

**Architecture:** Tauri 2（Rust 核心 + React/TypeScript 前端）。前端只通过 `invoke` 调用 Rust 命令；Rust 存储层是唯一读写磁盘的模块，正文存 `.md`、元数据存 SQLite、AI 结果只写元数据、原始正文不可变。AI 客户端为 OpenAI 兼容 HTTP 接口（默认 DeepSeek），接口抽象以支持后续本地模型。

**Tech Stack:** Tauri 2、React 19 + TypeScript + Vite、Tailwind CSS、Milkdown Crepe（所见即所得 Markdown）、rusqlite（bundled）、reqwest（rustls）、serde/serde_json、chrono、tokio、tauri-plugin-global-shortcut、tauri-plugin-dialog、tauri-plugin-single-instance、Vitest + React Testing Library。

**调研依据（避免重复造轮子）:** Glyph（Tauri 2 + React + SQLite + Markdown-first）验证了整体架构与"图片粘贴 → Rust 原子写入"模式；tfo 验证了"快捷键呼出 + 文本文件存储"的轻量路线；DeepSeek 官方 JSON Mode（`response_format: json_object`）用于结构化输出；Milkdown Crepe 提供开箱即用的 Markdown 所见即所得编辑器。**注意：Glyph 是 AGPL 协议，本计划只借鉴模式（原子写入、路径校验、派生索引），不复制其代码。**

---

## 文件结构总览

```
quick-note/
├── package.json                    # 前端依赖与脚本
├── vite.config.ts
├── tsconfig.json
├── index.html
├── src/
│   ├── main.tsx
│   ├── App.tsx                     # 捕获窗 / 主窗按 window.label 分流
│   ├── types.ts                    # 前后端共享的 DTO 类型
│   ├── lib/
│   │   ├── tauri.ts                # 所有 invoke 封装（唯一 IPC 出口）
│   │   ├── paste.ts                # 剪贴板图片提取 + 图片插入编辑器
│   │   └── autosave.ts             # 防抖自动保存 hook
│   ├── components/
│   │   ├── capture/CaptureWindow.tsx
│   │   ├── capture/TodayStrip.tsx
│   │   ├── main/MainWindow.tsx
│   │   ├── main/Sidebar.tsx
│   │   ├── main/NoteList.tsx
│   │   ├── main/NoteDetail.tsx
│   │   ├── review/ReviewPage.tsx
│   │   ├── kb/KnowledgeBase.tsx
│   │   └── settings/SettingsPage.tsx
│   └── tests/                      # Vitest 组件/逻辑测试
└── src-tauri/
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── capabilities/default.json
    └── src/
        ├── main.rs
        ├── lib.rs                  # 窗口管理、快捷键、托盘、定时器启动
        ├── state.rs                # AppState（Mutex<Storage>, Mutex<Config>, AiClient）
        ├── config.rs               # config.json 读写 + 数据目录引导
        ├── paths.rs                # join_under 路径安全 + ID→路径推导
        ├── io_atomic.rs            # 原子写入
        ├── storage/
        │   ├── mod.rs              # Storage 门面 + 连接管理 + 迁移
        │   ├── db.rs               # schema 与 migrations（PRAGMA user_version）
        │   ├── notes.rs            # notes 表 CRUD + body_index
        │   ├── categories.rs       # categories 表 + 上限规则
        │   ├── suggestions.rs      # suggestions/daily_summaries
        │   └── rebuild.rs          # 从 .md 重建索引
        ├── ai/
        │   ├── mod.rs              # AiProvider trait
        │   ├── openai.rs           # OpenAI 兼容 HTTP 实现
        │   ├── prompt.rs           # 系统提示词 + 请求体组装
        │   ├── parse.rs            # JSON 解析 + 校验 + 修复重试
        │   └── schedule.rs         # 每晚定时器
        ├── commands.rs             # 全部 tauri::command
        └── errors.rs               # 统一错误类型（序列化为 {error}）
```

## 全局约定

- 记录 ID：`yyyyMMdd-HHmmss-xxxx`（4 位小写字母数字随机后缀），例如 `20260808-153012-ab12`。
- 路径推导：`notes/YYYY/MM/<id>.md`；附件目录 `attachments/YYYY/MM/<id>/`；Markdown 内图片引用使用相对应用根目录的路径 `attachments/YYYY/MM/<id>/img_001.png`。
- 原始不可变：任何 AI 整理/编辑元数据操作都不写 `.md`；`notes.body_index` 是派生索引列（保存时填充、`rebuild_index` 重建），仅用于搜索。
- 错误约定：Rust 命令返回 `Result<T, String>`，`errors.rs` 提供 `AppError` 到字符串的转换；前端统一 `toast(error)`。
- 提交约定：每个任务末尾 `git commit`，信息为 `feat: ...` / `test: ...` / `chore: ...`。
- 分类上限：启用且非"其他"的分类数 ≤ 10；AI 提议新分类在确认时若超限则归"其他"。

---

## Task 1: 脚手架（Tauri 2 + React + TS）

**Files:**
- Create: 由 create-tauri-app 生成的整个项目骨架（package.json、vite.config.ts、tsconfig.json、index.html、src/、src-tauri/）
- Modify: `.gitignore`（补充 `src-tauri/target/` 已有，确认 `node_modules/`）

- [ ] **Step 1: 用官方脚手架生成项目到临时目录**

```bash
cd "$TEMP"
npm create tauri-app@latest quicknote-scaffold -- --template react-ts --manager npm --yes
```

Expected: `quicknote-scaffold/` 目录生成，包含 `src-tauri/`、`package.json`。

- [ ] **Step 2: 把脚手架文件移入项目根目录**

```powershell
$scaffold = Join-Path $env:TEMP 'quicknote-scaffold'
Get-ChildItem $scaffold -Force | Where-Object { $_.Name -notin @('.git') } | Move-Item -Destination 'D:\work\codexwork\quick-note' -Force
```

Expected: 根目录出现 `src/`、`src-tauri/`、`package.json` 等；`docs/`、`.gitignore` 不受影响。

- [ ] **Step 3: 安装依赖并验证 dev 构建**

```bash
npm install
npm run tauri dev
```

Expected: 窗口能打开显示 React 模板页。验证后 Ctrl+C 退出。

- [ ] **Step 4: 提交**

```bash
git add -A
git commit -m "chore: scaffold Tauri 2 + React TS app"
```

---

## Task 2: 路径安全与原子写入（Rust）

**Files:**
- Create: `src-tauri/src/paths.rs`
- Create: `src-tauri/src/io_atomic.rs`
- Create: `src-tauri/src/errors.rs`
- Test: `src-tauri/src/paths.rs`（内联 `#[cfg(test)]`）、`src-tauri/src/io_atomic.rs`

- [ ] **Step 1: 写失败测试**

在 `src-tauri/src/paths.rs` 写入：

```rust
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
        assert_eq!(p, Path::new("D:/notes/attachments/2026/08/20260808-153012-ab12"));
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

```bash
cd src-tauri && cargo test paths
```

Expected: 编译失败，`join_under` / `derive_note_path` 未定义。

- [ ] **Step 3: 实现 paths.rs**

```rust
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
    root.join("notes").join(year).join(month).join(format!("{id}.md"))
}

/// 从记录 ID 推导附件目录：attachments/YYYY/MM/<id>/
pub fn derive_attachment_dir(root: &Path, id: &str) -> PathBuf {
    let (year, month) = id_year_month(id);
    root.join("attachments").join(year).join(month).join(id)
}

fn id_year_month(id: &str) -> (&str, &str) {
    (&id[0..4], &id[4..6])
}
```

- [ ] **Step 4: 运行测试确认通过**

```bash
cd src-tauri && cargo test paths
```

Expected: 5 个测试全部 PASS。

- [ ] **Step 5: 实现 io_atomic.rs（临时文件 + rename）**

```rust
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
```

- [ ] **Step 6: 实现 errors.rs**

```rust
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AppError {
    pub error: String,
}

impl AppError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { error: msg.into() }
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        Self::new(format!("database error: {e}"))
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::new(format!("io error: {e}"))
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        Self::new(format!("network error: {e}"))
    }
}

pub type AppResult<T> = Result<T, AppError>;
```

- [ ] **Step 7: 运行全部 Rust 测试并提交**

```bash
cd src-tauri && cargo test
git add src-tauri/src
git commit -m "feat: safe paths and atomic file writes"
```

Expected: `paths`、`io_atomic` 测试通过。

---

## Task 3: SQLite schema、迁移与 notes CRUD（Rust）

**Files:**
- Create: `src-tauri/src/storage/mod.rs`
- Create: `src-tauri/src/storage/db.rs`
- Create: `src-tauri/src/storage/notes.rs`
- Create: `src-tauri/src/storage/categories.rs`
- Create: `src-tauri/src/storage/suggestions.rs`
- Test: 各模块内联 `#[cfg(test)]`

- [ ] **Step 1: 写 notes 的失败测试（storage/notes.rs）**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{open_in_memory, Storage};

    fn storage() -> Storage {
        let conn = open_in_memory().unwrap();
        Storage::new(conn)
    }

    #[test]
    fn insert_and_get_note() {
        let s = storage();
        let meta = s.insert_note("20260808-153012-ab12", "周报", "明天交周报", "2026-08-08T15:30:12+08:00").unwrap();
        assert_eq!(meta.id, "20260808-153012-ab12");
        assert_eq!(meta.title, "周报");
        let got = s.get_note("20260808-153012-ab12").unwrap().unwrap();
        assert_eq!(got.body_index, "明天交周报");
    }

    #[test]
    fn update_note_metadata_keeps_body_index() {
        let s = storage();
        s.insert_note("20260808-153012-ab12", "周报", "明天交周报", "2026-08-08T15:30:12+08:00").unwrap();
        s.update_ai_meta("20260808-153012-ab12", Some(1), Some("明天下午3点交周报"), r#"["周报"]"#).unwrap();
        let got = s.get_note("20260808-153012-ab12").unwrap().unwrap();
        assert_eq!(got.ai_status, "confirmed");
        assert_eq!(got.summary.as_deref(), Some("明天下午3点交周报"));
        assert_eq!(got.body_index, "明天交周报");
    }

    #[test]
    fn list_notes_filters_by_date() {
        let s = storage();
        s.insert_note("20260808-153012-ab12", "a", "x", "2026-08-08T15:30:12+08:00").unwrap();
        s.insert_note("20260809-100000-cd34", "b", "y", "2026-08-09T10:00:00+08:00").unwrap();
        let today = s.list_notes_by_date("2026-08-08").unwrap();
        assert_eq!(today.len(), 1);
        assert_eq!(today[0].id, "20260808-153012-ab12");
    }
}
```

- [ ] **Step 2: 运行确认失败**

```bash
cd src-tauri && cargo test storage::notes
```

Expected: 编译失败（`Storage` 未定义）。

- [ ] **Step 3: 实现 storage/db.rs（schema + 迁移）**

```rust
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
```

- [ ] **Step 4: 实现 storage/mod.rs 与 notes.rs**

`storage/mod.rs`：

```rust
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
```

`storage/notes.rs`（完整）：

```rust
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
    get_note(conn, id)?.ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)
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

pub fn list_notes_by_category(conn: &Connection, category_id: i64) -> rusqlite::Result<Vec<NoteMeta>> {
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
        params![category_id, summary, keywords, chrono::Utc::now().to_rfc3339(), id],
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
    Ok(NoteWithBody { meta, body_index: r.get(9)? })
}
```

`storage/categories.rs`（完整）：

```rust
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
        return Err(rusqlite::Error::QueryReturnedNoRows); // 调用方用 AppError 转成明确文案
    }
    conn.execute(
        "INSERT INTO categories (name, origin, enabled, sort_order, created_at)
         VALUES (?1, ?2, 1, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM categories), ?3)",
        params![name, origin, chrono::Utc::now().to_rfc3339()],
    )?;
    let id = id_by_name(conn, name)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    list(conn)?.into_iter().find(|c| c.id == id).ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn rename(conn: &Connection, id: i64, new_name: &str) -> rusqlite::Result<()> {
    conn.execute("UPDATE categories SET name = ?1 WHERE id = ?2", params![new_name, id])?;
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
```

`storage/suggestions.rs`（完整）：

```rust
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

pub fn insert_batch(conn: &Connection, rows: &[(String, Option<String>, Option<String>, Option<String>, Option<String>, String)]) -> rusqlite::Result<()> {
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
    let rows = stmt.query_map(params![date], |r| {
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
    })?;
    rows.collect()
}

pub fn set_status(conn: &Connection, id: i64, status: &str) -> rusqlite::Result<()> {
    conn.execute("UPDATE suggestions SET status = ?1 WHERE id = ?2", params![status, id])?;
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
```

- [ ] **Step 5: 补一个 categories 测试（上限与兜底）**

```rust
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
        let name = list(&conn).unwrap().into_iter().find(|c| c.id == fallback_id(&conn).unwrap()).unwrap().name;
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
```

- [ ] **Step 6: 运行全部 storage 测试**

```bash
cd src-tauri && cargo test storage
```

Expected: notes/categories/suggestions 相关测试全部 PASS。

- [ ] **Step 7: 提交**

```bash
git add src-tauri/src/storage
git commit -m "feat: sqlite schema, notes, categories with cap, suggestions"
```

---

## Task 4: 配置与数据目录（Rust）

**Files:**
- Create: `src-tauri/src/config.rs`
- Test: `src-tauri/src/config.rs`

- [ ] **Step 1: 失败测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn default_config_when_missing() {
        let dir = temp_dir().join(format!("qncfg-{}", std::process::id()));
        let cfg = load(&dir).unwrap();
        assert_eq!(cfg.hotkey, "Alt+Shift+N");
        assert_eq!(cfg.ai.base_url, "https://api.deepseek.com/v1");
        assert_eq!(cfg.org_time, "22:00");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = temp_dir().join(format!("qncfg2-{}", std::process::id()));
        let mut cfg = load(&dir).unwrap();
        cfg.ai.api_key = "sk-test".into();
        cfg.org_time = "23:30".into();
        save(&dir, &cfg).unwrap();
        let loaded = load(&dir).unwrap();
        assert_eq!(loaded.ai.api_key, "sk-test");
        assert_eq!(loaded.org_time, "23:30");
        std::fs::remove_dir_all(&dir).ok();
    }
}
```

- [ ] **Step 2: 实现 config.rs**

```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::io_atomic::atomic_write;
use crate::paths::join_under;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub base_url: String,
    pub model: String,
    pub api_key: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.deepseek.com/v1".into(),
            model: "deepseek-chat".into(),
            api_key: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub data_dir: PathBuf,
    pub hotkey: String,
    pub org_time: String, // "HH:MM"，24 小时制
    pub auto_org_enabled: bool,
    pub last_org_date: Option<String>, // 上一次成功整理的日期 yyyy-MM-dd
    pub ai: AiConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            hotkey: "Alt+Shift+N".into(),
            org_time: "22:00".into(),
            auto_org_enabled: true,
            last_org_date: None,
            ai: AiConfig::default(),
        }
    }
}

pub fn default_data_dir() -> PathBuf {
    dirs_home().join("Documents").join("QuickNote")
}

fn dirs_home() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn config_path(data_dir: &Path) -> PathBuf {
    data_dir.join("config.json")
}

pub fn load(data_dir: &Path) -> Result<Config, String> {
    let path = config_path(data_dir);
    if !path.exists() {
        let mut cfg = Config::default();
        cfg.data_dir = data_dir.to_path_buf();
        save(data_dir, &cfg)?;
        return Ok(cfg);
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| format!("读取配置失败: {e}"))?;
    let mut cfg: Config = serde_json::from_str(&raw).map_err(|e| format!("配置格式错误: {e}"))?;
    cfg.data_dir = data_dir.to_path_buf();
    Ok(cfg)
}

pub fn save(data_dir: &Path, cfg: &Config) -> Result<(), String> {
    let path = join_under(data_dir, Path::new("config.json")).map_err(|e| e)?;
    let raw = serde_json::to_string_pretty(cfg).map_err(|e| format!("序列化配置失败: {e}"))?;
    atomic_write(&path, raw.as_bytes()).map_err(|e| format!("写入配置失败: {e}"))?;
    Ok(())
}
```

注意：`dirs_home` 是简化实现；正式实现用 `dirs` crate（`dirs::document_dir()`），在 Step 3 中切换。

- [ ] **Step 3: 切换到 `dirs` crate**

在 `src-tauri/Cargo.toml` 加 `dirs = "6"`，并把 `dirs_home()` 改为：

```rust
fn dirs_home() -> PathBuf {
    dirs::document_dir().unwrap_or_else(|| PathBuf::from("."))
}
```

- [ ] **Step 4: 测试并提交**

```bash
cd src-tauri && cargo test config
git add src-tauri/src/config.rs src-tauri/Cargo.toml
git commit -m "feat: config load/save with first-run data dir"
```

---

## Task 5: 记录 ID 与图片落盘工具（Rust）

**Files:**
- Create: `src-tauri/src/notes_fs.rs`（正文/附件的文件层：保存、读取、图片校验）
- Test: `src-tauri/src/notes_fs.rs`

- [ ] **Step 1: 失败测试**

```rust
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
```

- [ ] **Step 2: 实现 notes_fs.rs**

```rust
use crate::io_atomic::atomic_write;
use crate::paths::{derive_attachment_dir, derive_note_path, join_under};
use std::path::Path;

pub fn generate_id(now: &chrono::DateTime<chrono::FixedOffset>) -> String {
    use rand::Rng;
    let suffix: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(4)
        .map(char::from)
        .collect::<String>()
        .to_ascii_lowercase();
    format!("{}{}", now.format("%Y%m%d-%H%M%S"), format!("-{suffix}"))
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

fn is_safe_filename(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.starts_with('.')
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains(':')
}

pub fn save_pasted_image(
    root: &Path,
    note_id: &str,
    original_filename: &str,
    bytes: &[u8],
    mime: &str,
) -> Result<String, String> {
    let ext = extension_for_mime(mime).ok_or_else(|| format!("不支持的图片类型: {mime}"))?;
    let _ = original_filename; // 只保留扩展名，统一命名为 img_NN
    let dir = derive_attachment_dir(root, note_id);

    let mut index = 1usize;
    let rel = loop {
        let name = format!("img_{:03}.{ext}", index);
        let rel = format!("attachments/{}/{}", note_id_path_part(note_id), name);
        let abs = join_under(root, Path::new(&rel)).map_err(|e| e)?;
        if !abs.exists() {
            atomic_write(&abs, bytes).map_err(|e| format!("保存图片失败: {e}"))?;
            break rel;
        }
        index += 1;
        if index > 999 {
            return Err("图片数量过多".into());
        }
    };
    let _ = dir;
    Ok(rel)
}

fn note_id_path_part(id: &str) -> String {
    let (y, m) = (&id[0..4], &id[4..6]);
    format!("{y}/{m}/{id}")
}
```

注意：`save_pasted_image` 中 `dir` 变量仅用于说明附件目录，正式实现直接使用 `derive_attachment_dir` 创建目录；`is_safe_filename` 保留供后续拖拽文件（保留原始文件名）使用。

- [ ] **Step 3: 运行测试并提交**

```bash
cd src-tauri && cargo test notes_fs
git add src-tauri/src/notes_fs.rs src-tauri/Cargo.toml
git commit -m "feat: note id, markdown files, pasted image storage"
```

Expected: 3 个测试 PASS（`Cargo.toml` 需追加 `rand = "0.8"`）。

---

## Task 6: 命令层：捕获、列表、重建索引（Rust + 前端类型）

**Files:**
- Create: `src-tauri/src/state.rs`
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`（注册命令与状态）
- Create: `src/types.ts`
- Create: `src/lib/tauri.ts`

- [ ] **Step 1: 实现 state.rs**

```rust
use std::path::PathBuf;
use std::sync::Mutex;

use crate::ai::AiClient;
use crate::config::Config;
use crate::storage::Storage;

pub struct AppState {
    pub config: Mutex<Config>,
    pub storage: Mutex<Storage>,
    pub data_dir: PathBuf,
    pub ai: AiClient,
}

impl AppState {
    pub fn new(config: Config, storage: Storage, ai: AiClient) -> Self {
        let data_dir = config.data_dir.clone();
        Self { config: Mutex::new(config), storage: Mutex::new(storage), data_dir, ai }
    }
}
```

- [ ] **Step 2: 实现 commands.rs（捕获与列表部分）**

```rust
use serde::Serialize;
use tauri::State;

use crate::errors::{AppError, AppResult};
use crate::notes_fs::{generate_id, save_note_file, save_pasted_image, title_from_markdown};
use crate::state::AppState;
use crate::storage::notes::{self as notes_db};

#[derive(serde::Deserialize)]
pub struct PastedImage {
    pub filename: String,
    pub mime: String,
    pub bytes: Vec<u8>,
}

#[derive(Serialize)]
pub struct SaveNoteResult {
    pub id: String,
    pub markdown_path: String,
    pub image_refs: Vec<String>,
}

#[tauri::command]
pub fn save_note(
    state: State<AppState>,
    markdown: String,
    images: Vec<PastedImage>,
) -> AppResult<SaveNoteResult> {
    let now = chrono::Local::now();
    let id = generate_id(&now.fixed_offset());
    let created = now.to_rfc3339();
    let title = title_from_markdown(&markdown);

    let mut image_refs = Vec::new();
    let root = state.data_dir.clone();
    for img in images {
        let rel = save_pasted_image(&root, &id, &img.filename, &img.bytes, &img.mime)
            .map_err(AppError::new)?;
        image_refs.push(rel);
    }

    save_note_file(&root, &id, &markdown).map_err(AppError::new)?;
    let storage = state.storage.lock().unwrap();
    let _ = notes_db::insert_note(storage.conn(), &id, &title, &markdown, &created)?;

    Ok(SaveNoteResult {
        id,
        markdown_path: format!("notes/{}/{}/{}.md", &id[0..4], &id[4..6], id),
        image_refs,
    })
}

#[tauri::command]
pub fn list_notes(state: State<AppState>, date: Option<String>) -> AppResult<Vec<notes_db::NoteMeta>> {
    let storage = state.storage.lock().unwrap();
    let rows = match date {
        Some(d) => notes_db::list_notes_by_date(storage.conn(), &d)?,
        None => notes_db::list_all(storage.conn())?,
    };
    Ok(rows)
}

#[tauri::command]
pub fn get_note(state: State<AppState>, id: String) -> AppResult<Option<String>> {
    let storage = state.storage.lock().unwrap();
    let exists = notes_db::get_note(storage.conn(), &id)?;
    drop(storage);
    if exists.is_none() {
        return Ok(None);
    }
    Ok(Some(crate::notes_fs::read_note_file(&state.data_dir, &id).map_err(AppError::new)?))
}
```

- [ ] **Step 3: 实现 rebuild.rs 并在 commands 中注册 `rebuild_index`**

```rust
use crate::errors::AppResult;
use crate::notes_fs::title_from_markdown;
use crate::paths::{derive_note_path, join_under};
use crate::storage::notes as notes_db;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// 扫描 notes/ 下所有 .md，重建 notes 表（不删除已确认的元数据行；只补缺失行并刷新 body_index）。
pub fn rebuild(conn: &Connection, root: &Path) -> AppResult<(usize, usize)> {
    let notes_root = join_under(root, Path::new("notes")).map_err(|e| crate::errors::AppError::new(e))?;
    let mut found = 0usize;
    let mut repaired = 0usize;
    for entry in walk(&notes_root)? {
        let stem = entry.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
        if !is_valid_id(stem) {
            continue;
        }
        found += 1;
        let body = std::fs::read_to_string(&entry)
            .map_err(|e| crate::errors::AppError::new(format!("读取 {entry:?} 失败: {e}")))?;
        let exists = notes_db::get_note(conn, stem)?.is_some();
        let created = id_to_rfc3339(stem);
        if !exists {
            notes_db::insert_note(conn, stem, &title_from_markdown(&body), &body, &created)?;
            repaired += 1;
        } else {
            notes_db::refresh_body_index(conn, stem, &body)?;
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
        &id[0..4], &id[4..6], &id[6..8], &id[9..11], &id[11..13], &id[13..15]
    )
}
```

在 `storage/notes.rs` 追加：

```rust
pub fn refresh_body_index(conn: &Connection, id: &str, body: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE notes SET body_index = ?1, updated_at = ?2 WHERE id = ?3",
        params![body, chrono::Utc::now().to_rfc3339(), id],
    )?;
    Ok(())
}
```

在 `commands.rs` 追加：

```rust
#[tauri::command]
pub fn rebuild_index(state: State<AppState>) -> AppResult<(usize, usize)> {
    let storage = state.storage.lock().unwrap();
    crate::storage::rebuild::rebuild(storage.conn(), &state.data_dir)
}
```

- [ ] **Step 4: 在 lib.rs 注册命令并初始化状态**

先创建最小可编译的 `ai` 模块占位（Task 7 会替换为真实客户端）：

`src-tauri/src/ai/mod.rs`（占位）：

```rust
pub struct AiClient {
    #[allow(dead_code)]
    base_url: String,
    #[allow(dead_code)]
    model: String,
    #[allow(dead_code)]
    api_key: String,
}

impl AiClient {
    pub fn new(base_url: String, model: String, api_key: String) -> Self {
        Self { base_url, model, api_key }
    }
}
```

`lib.rs`（骨架，后续任务继续扩展）：

```rust
mod ai;
mod commands;
mod config;
mod errors;
mod io_atomic;
mod notes_fs;
mod paths;
mod state;
mod storage;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let data_dir = config::default_data_dir();
    let cfg = config::load(&data_dir).expect("配置初始化失败");
    std::fs::create_dir_all(&cfg.data_dir).expect("数据目录创建失败");
    let storage = storage::Storage::open(&cfg.data_dir.join("quicknote.db")).expect("数据库初始化失败");
    let ai = ai::AiClient::new(cfg.ai.base_url.clone(), cfg.ai.model.clone(), cfg.ai.api_key.clone());
    let state = AppState::new(cfg, storage, ai);

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::save_note,
            commands::list_notes,
            commands::get_note,
            commands::rebuild_index,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 5: 写前端类型与 invoke 封装**

`src/types.ts`：

```ts
export type AiStatus = 'pending' | 'suggested' | 'confirmed' | 'skipped' | 'failed';

export interface NoteMeta {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
  ai_status: AiStatus;
  category_id: number | null;
  summary: string | null;
  keywords: string | null; // JSON 数组字符串
  tags: string | null;
}

export interface PastedImage {
  filename: string;
  mime: string;
  bytes: number[];
}

export interface SaveNoteResult {
  id: string;
  markdown_path: string;
  image_refs: string[];
}

export interface Category {
  id: number;
  name: string;
  origin: 'builtin' | 'ai' | 'user';
  enabled: boolean;
  sort_order: number;
  created_at: string;
}
```

`src/lib/tauri.ts`：

```ts
import { invoke } from '@tauri-apps/api/core';
import type { Category, NoteMeta, PastedImage, SaveNoteResult } from '../types';

export const api = {
  saveNote: (markdown: string, images: PastedImage[]) =>
    invoke<SaveNoteResult>('save_note', { markdown, images }),
  listNotes: (date?: string) => invoke<NoteMeta[]>('list_notes', { date: date ?? null }),
  getNote: (id: string) => invoke<string | null>('get_note', { id }),
  rebuildIndex: () => invoke<[number, number]>('rebuild_index'),
};
```

- [ ] **Step 6: cargo check + tsc 验证并提交**

```bash
cd src-tauri && cargo check
cd .. && npx tsc --noEmit
git add -A
git commit -m "feat: capture/list commands, rebuild index, frontend api wrappers"
```

Expected: 编译通过（`AiClient` 尚未实现，Task 7 提供最小占位实现）。

---

## Task 7: AI 客户端、提示词与解析（Rust）

**Files:**
- Create: `src-tauri/src/ai/mod.rs`
- Create: `src-tauri/src/ai/openai.rs`
- Create: `src-tauri/src/ai/prompt.rs`
- Create: `src-tauri/src/ai/parse.rs`
- Create: `src-tauri/src/ai/schedule.rs`（Task 8 使用）
- Test: `src-tauri/src/ai/parse.rs`

- [ ] **Step 1: 失败测试（parse.rs）**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_response() {
        let raw = r#"{
            "notes": [
                {"note_id": "20260808-153012-ab12", "category": "待办",
                 "new_category_proposal": null, "summary": "明天交周报", "keywords": ["周报"]}
            ],
            "daily_summary": "今天主要是周报"
        }"#;
        let parsed = parse_organization(raw).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].note_id, "20260808-153012-ab12");
        assert_eq!(parsed[0].category.as_deref(), Some("待办"));
    }

    #[test]
    fn rejects_unknown_note_id() {
        let raw = r#"{"notes": [{"note_id": "nope", "category": "待办"}], "daily_summary": ""}"#;
        assert!(parse_organization(raw).is_err());
    }

    #[test]
    fn rejects_missing_notes_field() {
        assert!(parse_organization("{}").is_err());
    }
}
```

- [ ] **Step 2: 实现 parse.rs**

```rust
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Deserialize)]
pub struct NoteSuggestion {
    pub note_id: String,
    pub category: Option<String>,
    #[serde(default)]
    pub new_category_proposal: Option<String>,
    pub summary: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrganizationResponse {
    pub notes: Vec<NoteSuggestion>,
    #[serde(default)]
    pub daily_summary: Option<String>,
}

pub fn parse_organization(raw: &str) -> Result<Vec<NoteSuggestion>, String> {
    let resp: OrganizationResponse =
        serde_json::from_str(raw).map_err(|e| format!("AI 返回不是合法 JSON: {e}"))?;
    if resp.notes.is_empty() {
        return Err("AI 未返回任何记录".into());
    }
    Ok(resp.notes)
}

pub fn validate_against_batch(
    suggestions: &[NoteSuggestion],
    batch_ids: &[String],
) -> Result<(), String> {
    let allowed: HashSet<&str> = batch_ids.iter().map(String::as_str).collect();
    for s in suggestions {
        if !allowed.contains(s.note_id.as_str()) {
            return Err(format!("AI 返回了未知记录 ID: {}", s.note_id));
        }
    }
    Ok(())
}
```

- [ ] **Step 3: 实现 prompt.rs 与 openai.rs、mod.rs**

`prompt.rs`：

```rust
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

pub fn build_user_payload(notes: &[(String, String)], daily_summary_hint: &str) -> serde_json::Value {
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
```

`openai.rs`：

```rust
use serde_json::{json, Value};

pub struct OpenAiClient {
    base_url: String,
    model: String,
    api_key: String,
    http: reqwest::Client,
}

impl OpenAiClient {
    pub fn new(base_url: String, model: String, api_key: String) -> Self {
        Self {
            base_url,
            model,
            api_key,
            http: reqwest::Client::new(),
        }
    }

    pub async fn chat_json(
        &self,
        system: &str,
        user: Value,
    ) -> Result<Value, crate::errors::AppError> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let body = json!({
            "model": self.model,
            "temperature": 0,
            "response_format": { "type": "json_object" },
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": serde_json::to_string(&user).map_err(|e| crate::errors::AppError::new(e.to_string()))? }
            ]
        });
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(crate::errors::AppError::new(format!(
                "AI API 错误 {status}: {}",
                text.chars().take(300).collect::<String>()
            )));
        }
        let json: Value = resp.json().await?;
        json.get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or_else(|| crate::errors::AppError::new("AI 响应缺少 content".to_string()))
            .map(|s| serde_json::from_str(s).unwrap_or_else(|_| json!({"raw": s})))
    }
}
```

`mod.rs`：

```rust
pub mod openai;
pub mod parse;
pub mod prompt;
pub mod schedule;

pub use openai::OpenAiClient as AiClient;
```

注意：`AiClient` 目前是具体类型；后续本地模型接入时改为 trait 对象（接口不变，仅替换 `state.rs` 的字段类型）。

- [ ] **Step 4: 运行测试**

```bash
cd src-tauri && cargo test ai::parse
```

Expected: parse 3 个测试 PASS。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/ai src-tauri/Cargo.toml
git commit -m "feat: openai-compatible ai client with json mode and validation"
```

（`Cargo.toml` 需追加 `reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }`、`tokio = { version = "1", features = ["time", "macros"] }`。）

---

## Task 8: AI 整理调度与确认落地（Rust）

**Files:**
- Modify: `src-tauri/src/ai/schedule.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/ai/schedule.rs`（时间判断纯函数）

- [ ] **Step 1: 失败测试（schedule.rs）**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, Timelike};

    #[test]
    fn due_when_past_org_time_and_not_ran_today() {
        let now = Local::now().with_hour(23).unwrap().with_minute(0).unwrap();
        assert!(is_due(now, "22:00", None));
        assert!(!is_due(now, "22:00", Some(now.format("%Y-%m-%d").to_string())));
    }

    #[test]
    fn not_due_before_org_time() {
        let now = Local::now().with_hour(21).unwrap().with_minute(0).unwrap();
        assert!(!is_due(now, "22:00", None));
    }
}
```

- [ ] **Step 2: 实现 schedule.rs**

```rust
use chrono::{DateTime, Local};

pub fn is_due(now: DateTime<Local>, org_time: &str, last_org_date: Option<&str>) -> bool {
    let today = now.format("%Y-%m-%d").to_string();
    if last_org_date == Some(today.as_str()) {
        return false;
    }
    let Some((h, m)) = parse_hhmm(org_time) else { return false };
    (now.hour() as u32, now.minute() as u32) >= (h, m)
}

fn parse_hhmm(s: &str) -> Option<(u32, u32)> {
    let (h, m) = s.split_once(':')?;
    Some((h.parse().ok()?, m.parse().ok()?))
}
```

- [ ] **Step 3: 实现整理主流程 `run_organization`（commands.rs）**

```rust
use crate::ai::parse::{parse_organization, validate_against_batch};
use crate::ai::prompt::{build_system_prompt, build_user_payload};
use crate::storage::categories as categories_db;
use crate::storage::suggestions as suggestions_db;

#[derive(serde::Serialize)]
pub struct OrgRunResult {
    pub processed: usize,
    pub suggested: usize,
    pub failed: Vec<String>,
}

#[tauri::command]
pub async fn run_ai_org(state: State<'_, AppState>) -> AppResult<OrgRunResult> {
    let (cfg, batch, categories) = {
        let cfg = state.config.lock().unwrap();
        if cfg.ai.api_key.is_empty() {
            return Err(AppError::new("请先在设置中配置 API Key"));
        }
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let storage = state.storage.lock().unwrap();
        let batch = notes_db::pending_ids(storage.conn(), &date)?;
        let cats: Vec<String> = categories_db::list(storage.conn())?
            .into_iter()
            .filter(|c| c.enabled)
            .map(|c| c.name)
            .collect();
        (cfg.clone(), batch, cats)
    };

    if batch.is_empty() {
        return Ok(OrgRunResult { processed: 0, suggested: 0, failed: vec![] });
    }

    let mut failed = Vec::new();
    let mut all_suggestions = Vec::new();
    for chunk in batch.chunks(30) {
        let mut notes_text = Vec::new();
        for id in chunk {
            let text = crate::notes_fs::read_note_file(&state.data_dir, id)
                .unwrap_or_else(|_| "[读取失败]".to_string());
            notes_text.push((id.clone(), text));
        }
        let system = build_system_prompt(&categories, 10);
        let user = build_user_payload(&notes_text, "请只输出 JSON");
        let mut attempt = 0;
        let parsed = loop {
            match state.ai.chat_json(&system, user.clone()).await {
                Ok(v) => {
                    let raw = v.to_string();
                    match parse_organization(&raw) {
                        Ok(p) => break p,
                        Err(_) if attempt == 0 => {
                            attempt += 1;
                            continue;
                        }
                        Err(e) => {
                            failed.push(format!("{}: {e}", chunk[0]));
                            break Vec::new();
                        }
                    }
                }
                Err(e) => {
                    failed.push(format!("{}: {e}", chunk[0]));
                    break Vec::new();
                }
            }
        };
        if let Err(e) = validate_against_batch(&parsed, chunk) {
            failed.push(e);
            continue;
        }
        all_suggestions.extend(parsed);
    }

    let storage = state.storage.lock().unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    let rows: Vec<(String, Option<String>, Option<String>, Option<String>, Option<String>, String)> =
        all_suggestions
            .iter()
            .map(|s| {
                (
                    s.note_id.clone(),
                    s.category.clone(),
                    s.new_category_proposal.clone(),
                    s.summary.clone(),
                    if s.keywords.is_empty() {
                        None
                    } else {
                        Some(serde_json::to_string(&s.keywords).unwrap_or_else(|_| "[]".into()))
                    },
                    now.clone(),
                )
            })
            .collect();
    suggestions_db::insert_batch(storage.conn(), &rows)?;
    for s in &all_suggestions {
        notes_db::set_ai_status(storage.conn(), &s.note_id, "suggested")?;
    }
    drop(storage);

    let mut cfg = state.config.lock().unwrap();
    cfg.last_org_date = Some(chrono::Local::now().format("%Y-%m-%d").to_string());
    crate::config::save(&state.data_dir, &cfg).map_err(AppError::new)?;

    Ok(OrgRunResult {
        processed: batch.len(),
        suggested: all_suggestions.len(),
        failed,
    })
}
```

- [ ] **Step 4: 实现确认落地命令**

```rust
#[tauri::command]
pub fn accept_suggestion(
    state: State<AppState>,
    suggestion_id: i64,
    category_name: Option<String>,
    summary: Option<String>,
    keywords: Option<String>,
) -> AppResult<()> {
    let storage = state.storage.lock().unwrap();
    let conn = storage.conn();
    let sug = suggestions_db::get(conn, suggestion_id)?.ok_or_else(|| AppError::new("建议不存在"))?;
    let cat_name = category_name.clone().unwrap_or_else(|| {
        sug.new_category_proposal
            .clone()
            .or(sug.ai_category.clone())
            .unwrap_or_else(|| "其他".to_string())
    });

    let category_id = match categories_db::id_by_name(conn, &cat_name)? {
        Some(id) => Some(id),
        None => {
            if categories_db::active_count(conn)? >= categories_db::MAX_ACTIVE_CATEGORIES {
                Some(categories_db::fallback_id(conn)?)
            } else {
                Some(categories_db::create(conn, &cat_name, "ai")?.id)
            }
        }
    };

    notes_db::update_ai_meta(conn, &sug.note_id, category_id, summary.as_deref(), keywords.as_deref())?;
    suggestions_db::set_status(conn, suggestion_id, "accepted")?;
    Ok(())
}

#[tauri::command]
pub fn skip_suggestion(state: State<AppState>, suggestion_id: i64) -> AppResult<()> {
    let storage = state.storage.lock().unwrap();
    let conn = storage.conn();
    let sug = suggestions_db::get(conn, suggestion_id)?.ok_or_else(|| AppError::new("建议不存在"))?;
    notes_db::set_ai_status(conn, &sug.note_id, "skipped")?;
    suggestions_db::set_status(conn, suggestion_id, "skipped")?;
    Ok(())
}

#[tauri::command]
pub fn accept_all(state: State<AppState>, date: String) -> AppResult<usize> {
    let storage = state.storage.lock().unwrap();
    let conn = storage.conn();
    let list = suggestions_db::list_by_date(conn, &date)?;
    for s in list.iter().filter(|s| s.status == "suggested") {
        let cat = s.ai_category.clone().unwrap_or_else(|| "其他".to_string());
        let category_id = categories_db::id_by_name(conn, &cat)?;
        notes_db::update_ai_meta(
            conn,
            &s.note_id,
            category_id,
            s.summary.as_deref(),
            s.keywords.as_deref(),
        )?;
        suggestions_db::set_status(conn, s.id, "accepted")?;
    }
    Ok(list.len())
}
```

`suggestions_db::get` 补充：

```rust
pub fn get(conn: &Connection, id: i64) -> rusqlite::Result<Option<Suggestion>> {
    let mut stmt = conn.prepare(
        "SELECT id, note_id, ai_category, new_category_proposal, summary, keywords, status, created_at
         FROM suggestions WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], |r| {
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
    })?;
    rows.next().transpose()
}
```

- [ ] **Step 5: 在 lib.rs 注册定时器与新增命令**

```rust
use tauri::Manager;

fn spawn_scheduler(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            let state = app.state::<AppState>();
            let due = {
                let cfg = state.config.lock().unwrap();
                crate::ai::schedule::is_due(
                    chrono::Local::now(),
                    &cfg.org_time,
                    cfg.last_org_date.as_deref(),
                ) && cfg.auto_org_enabled
            };
            if due {
                let _ = tauri::async_runtime::spawn(async move {
                    // 通过 app.handle 调用 commands::run_ai_org_inner 防止锁冲突
                    let _ = app.emit("ai-org-started", ());
                });
            }
        }
    });
}
```

说明：为避免闭包借用冲突，`run_ai_org` 应拆成 `async fn run_ai_org_inner(app: &AppHandle) -> AppResult<OrgRunResult>` 供命令与定时器共用；定时器运行后 `emit("ai-org-completed", result)`。此重构在实现时按此结构完成，命令签名保持不变。

- [ ] **Step 6: cargo test + cargo check + 提交**

```bash
cd src-tauri && cargo test ai::schedule && cargo check
git add -A
git commit -m "feat: nightly ai organization with retry, suggestions, confirm flow"
```

Expected: schedule 2 个测试 PASS，编译通过。

---

## Task 9: 捕获窗前端（Milkdown Crepe + 粘贴图片 + 自动保存）

**Files:**
- Modify: `package.json`（加 `@milkdown/crepe`、`@tauri-apps/api` 已有）
- Create: `src/lib/paste.ts`
- Create: `src/lib/autosave.ts`
- Create: `src/components/capture/CaptureWindow.tsx`
- Create: `src/components/capture/TodayStrip.tsx`
- Create: `src/tests/paste.test.ts`
- Modify: `src/App.tsx`（按 `window.__TAURI_INTERNALS__.metadata.currentWindow.label` 分流）

- [ ] **Step 1: 安装 Milkdown Crepe**

```bash
npm install @milkdown/crepe
```

- [ ] **Step 2: 写 paste 逻辑的失败测试**

`src/tests/paste.test.ts`：

```ts
import { describe, expect, it, vi } from 'vitest';
import { extractImageFromClipboardEvent } from '../lib/paste';

describe('extractImageFromClipboardEvent', () => {
  it('returns null when clipboard has no image', () => {
    const evt = { clipboardData: { items: [] } } as unknown as ClipboardEvent;
    expect(extractImageFromClipboardEvent(evt)).toBeNull();
  });

  it('returns file for image item', () => {
    const file = new File(['x'], 'a.png', { type: 'image/png' });
    const evt = {
      clipboardData: {
        items: [{ kind: 'file', type: 'image/png', getAsFile: () => file }],
      },
    } as unknown as ClipboardEvent;
    const got = extractImageFromClipboardEvent(evt);
    expect(got?.name).toBe('a.png');
    expect(got?.type).toBe('image/png');
  });
});
```

- [ ] **Step 3: 实现 paste.ts**

```ts
export function extractImageFromClipboardEvent(evt: ClipboardEvent): File | null {
  const items = evt.clipboardData?.items;
  if (!items) return null;
  for (const item of Array.from(items)) {
    if (item.kind === 'file' && item.type.startsWith('image/')) {
      return item.getAsFile();
    }
  }
  return null;
}

export async function fileToPastedImage(file: File) {
  const buf = await file.arrayBuffer();
  return {
    filename: file.name || 'clipboard-image',
    mime: file.type || 'image/png',
    bytes: Array.from(new Uint8Array(buf)),
  };
}

export function imageMarkdown(relPath: string): string {
  return `![图片](${relPath})`;
}
```

- [ ] **Step 4: 实现 autosave.ts**

```ts
import { useEffect, useRef, useState } from 'react';

export function useAutosave(getContent: () => string, onSave: (md: string) => Promise<void>, delay = 800) {
  const timer = useRef<number | null>(null);
  const [saving, setSaving] = useState(false);
  const [lastSavedAt, setLastSavedAt] = useState<number | null>(null);

  useEffect(() => {
    return () => {
      if (timer.current) window.clearTimeout(timer.current);
    };
  }, []);

  const flush = async () => {
    if (timer.current) {
      window.clearTimeout(timer.current);
      timer.current = null;
    }
    const md = getContent();
    if (!md.trim()) return;
    setSaving(true);
    try {
      await onSave(md);
      setLastSavedAt(Date.now());
    } finally {
      setSaving(false);
    }
  };

  const schedule = () => {
    if (timer.current) window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => void flush(), delay);
  };

  return { saving, lastSavedAt, schedule, flush };
}
```

- [ ] **Step 5: 实现 CaptureWindow.tsx**

```tsx
import { useEffect, useMemo, useRef, useState } from 'react';
import { Crepe } from '@milkdown/crepe';
import { api } from '../../lib/tauri';
import { extractImageFromClipboardEvent, fileToPastedImage, imageMarkdown } from '../../lib/paste';
import { useAutosave } from '../../lib/autosave';
import { TodayStrip } from './TodayStrip';

export function CaptureWindow() {
  const hostRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<Crepe | null>(null);
  const currentId = useRef<string | null>(null);
  const [savedCount, setSavedCount] = useState(0);

  const editor = useMemo(() => {
    if (!hostRef.current) return null;
    const crepe = new Crepe({
      root: hostRef.current,
      defaultValue: '',
      features: {
        imageBlock: true,
        toolbar: ['bold', 'italic', 'strike', 'code', 'image', 'link'],
      },
    });
    void crepe.create();
    return crepe;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    editorRef.current = editor;
  }, [editor]);

  const doSave = async (md: string) => {
    const result = await api.saveNote(md, []);
    currentId.current = result.id;
    setSavedCount((c) => c + 1);
  };

  const { saving, schedule, flush } = useAutosave(
    () => editorRef.current?.getMarkdown() ?? '',
    doSave,
  );

  useEffect(() => {
    const onPaste = async (evt: ClipboardEvent) => {
      const file = extractImageFromClipboardEvent(evt);
      if (!file) return;
      evt.preventDefault();
      const md = editorRef.current?.getMarkdown() ?? '';
      const result = await api.saveNote(md, [await fileToPastedImage(file)]);
      currentId.current = result.id;
      editorRef.current?.setMarkdown(`${md}\n${imageMarkdown(result.image_refs[0])}\n`);
    };
    window.addEventListener('paste', onPaste);
    return () => window.removeEventListener('paste', onPaste);
  }, []);

  const onBlur = () => void flush();

  return (
    <div className="h-screen flex flex-col" onBlur={onBlur}>
      <div ref={hostRef} className="flex-1 overflow-y-auto p-3" />
      <TodayStrip onRefresh={() => setSavedCount((c) => c + 1)} />
    </div>
  );
}
```

注意：粘贴图片时先保存正文+图片再更新编辑器，保证图片文件已落盘、引用路径有效；`Esc` 隐藏由 Rust 窗口事件处理，前端在 `beforeunload`/失焦时 flush。

- [ ] **Step 6: 实现 TodayStrip.tsx**

```tsx
import { useEffect, useState } from 'react';
import { api } from '../../lib/tauri';
import type { NoteMeta } from '../../types';

export function TodayStrip({ onRefresh }: { onRefresh?: () => void }) {
  const [expanded, setExpanded] = useState(false);
  const [notes, setNotes] = useState<NoteMeta[]>([]);

  const load = async () => {
    const today = new Date().toISOString().slice(0, 10);
    setNotes(await api.listNotes(today));
  };

  useEffect(() => {
    void load();
  }, [onRefresh]);

  return (
    <div className="border-t border-gray-200">
      <button
        className="w-full flex items-center gap-2 px-3 py-2 text-sm text-gray-600 hover:bg-gray-50"
        onClick={() => setExpanded((v) => !v)}
      >
        <span>📥 今日 {notes.length} 条</span>
        <span className="ml-auto">{expanded ? '▴' : '▾'}</span>
      </button>
      {expanded && (
        <ul className="max-h-40 overflow-y-auto border-t border-gray-100">
          {notes.map((n) => (
            <li key={n.id} className="px-3 py-2 text-sm text-gray-700 hover:bg-gray-50">
              {n.title} <span className="text-gray-400 text-xs">{n.created_at.slice(11, 16)}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
```

- [ ] **Step 7: 运行前端测试**

```bash
npx vitest run src/tests/paste.test.ts
```

Expected: PASS。

- [ ] **Step 8: 提交**

```bash
git add src package.json package-lock.json
git commit -m "feat: capture window with crepe editor, image paste, autosave, today strip"
```

---

## Task 10: 主窗口前端（侧边栏 + 列表 + 详情 + 知识库）

**Files:**
- Create: `src/components/main/MainWindow.tsx`
- Create: `src/components/main/Sidebar.tsx`
- Create: `src/components/main/NoteList.tsx`
- Create: `src/components/main/NoteDetail.tsx`
- Create: `src/components/kb/KnowledgeBase.tsx`
- Create: `src/tests/mainwindow.test.tsx`

- [ ] **Step 1: 写导航状态与列表渲染的组件测试**

`src/tests/mainwindow.test.tsx`：

```tsx
import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Sidebar, type SidebarSection } from '../components/main/Sidebar';

const sections: SidebarSection[] = [
  { key: 'inbox', label: '今日速记', icon: '📥' },
  { key: 'review', label: '整理建议', icon: '✨', badge: 3 },
  { key: 'kb', label: '知识库', icon: '📚' },
];

describe('Sidebar', () => {
  it('renders sections and badge', () => {
    render(<Sidebar sections={sections} active="inbox" onSelect={() => {}} />);
    expect(screen.getByText('今日速记')).toBeTruthy();
    expect(screen.getByText('3')).toBeTruthy();
  });

  it('calls onSelect with key', () => {
    const onSelect = vi.fn();
    render(<Sidebar sections={sections} active="inbox" onSelect={onSelect} />);
    screen.getByText('知识库').click();
    expect(onSelect).toHaveBeenCalledWith('kb');
  });
});
```

- [ ] **Step 2: 实现 Sidebar.tsx**

```tsx
export interface SidebarSection {
  key: string;
  label: string;
  icon: string;
  badge?: number;
}

export function Sidebar(props: {
  sections: SidebarSection[];
  active: string;
  onSelect: (key: string) => void;
}) {
  return (
    <nav className="w-44 shrink-0 bg-gray-50 border-r border-gray-200 flex flex-col py-3">
      <div className="px-4 pb-3 text-base font-bold text-gray-800">QuickNote</div>
      {props.sections.map((s) => (
        <button
          key={s.key}
          data-testid={`sidebar-${s.key}`}
          onClick={() => props.onSelect(s.key)}
          className={`flex items-center gap-2 px-4 py-2 text-sm ${
            props.active === s.key ? 'bg-blue-50 text-blue-700 font-medium' : 'text-gray-600 hover:bg-gray-100'
          }`}
        >
          <span>{s.icon}</span>
          <span>{s.label}</span>
          {s.badge ? (
            <span className="ml-auto bg-blue-500 text-white text-xs rounded-full px-1.5">{s.badge}</span>
          ) : null}
        </button>
      ))}
    </nav>
  );
}
```

- [ ] **Step 3: 实现 NoteList.tsx 与 NoteDetail.tsx**

`NoteList.tsx`：

```tsx
import { useEffect, useState } from 'react';
import { api } from '../../lib/tauri';
import type { NoteMeta } from '../../types';

export function NoteList(props: { date?: string; categoryId?: number; onOpen: (id: string) => void }) {
  const [notes, setNotes] = useState<NoteMeta[]>([]);
  const [q, setQ] = useState('');

  useEffect(() => {
    void api.listNotes(props.date).then(setNotes);
  }, [props.date, props.categoryId]);

  return (
    <div className="flex-1 overflow-y-auto">
      <input
        value={q}
        onChange={(e) => setQ(e.target.value)}
        placeholder="搜索记录…"
        className="m-3 w-[calc(100%-24px)] border border-gray-200 rounded-md px-3 py-1.5 text-sm"
      />
      {notes
        .filter((n) => !q || n.title.includes(q))
        .map((n) => (
          <button
            key={n.id}
            onClick={() => props.onOpen(n.id)}
            className="w-full text-left px-4 py-3 border-b border-gray-100 hover:bg-gray-50"
          >
            <div className="text-sm font-medium text-gray-800">{n.title}</div>
            <div className="text-xs text-gray-400">
              {n.created_at.slice(0, 16).replace('T', ' ')} · {n.ai_status}
            </div>
            {n.summary ? <div className="text-xs text-gray-500 mt-1">{n.summary}</div> : null}
          </button>
        ))}
    </div>
  );
}
```

`NoteDetail.tsx`：

```tsx
import { useEffect, useState } from 'react';
import { api } from '../../lib/tauri';
import { renderMarkdown } from '../../lib/markdown';

export function NoteDetail(props: { id: string }) {
  const [html, setHtml] = useState('');

  useEffect(() => {
    void api.getNote(props.id).then(async (md) => {
      if (md) setHtml(await renderMarkdown(md));
    });
  }, [props.id]);

  return (
    <article
      className="flex-1 overflow-y-auto p-6 prose prose-sm max-w-none"
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}
```

`src/lib/markdown.ts`（用 `marked` + DOMPurify；MVP 渲染只读，不引入重编辑器）：

```ts
import { marked } from 'marked';
import DOMPurify from 'dompurify';

export async function renderMarkdown(md: string): Promise<string> {
  const raw = await marked.parse(md, { async: false });
  return DOMPurify.sanitize(raw);
}
```

```bash
npm install marked dompurify
```

注意：图片相对路径需解析——在 `NoteDetail` 中把 `src="attachments/..."` 替换为 `convertFileSrc(join(dataDir, rel))`；MVP 通过 Tauri 自定义协议 `asset` 提供：`convertFileSrc(path)`。实现时在 `markdown.ts` 增加 `resolveAttachmentSrc(html, baseDir)`，并在 `NoteDetail` 中调用；baseDir 由 `get_settings` 命令（Task 12）提供。

- [ ] **Step 4: 实现 KnowledgeBase.tsx（B 布局：列表 + 右侧阅读）**

```tsx
import { useEffect, useState } from 'react';
import { api } from '../../lib/tauri';
import type { Category, NoteMeta } from '../../types';

export function KnowledgeBase() {
  const [notes, setNotes] = useState<NoteMeta[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [q, setQ] = useState('');

  useEffect(() => {
    void (async () => {
      const cats = await api.listCategories();
      const kb = cats.find((c: Category) => c.name === '知识库');
      const all = await api.listNotes();
      const inKb = kb ? all.filter((n) => n.category_id === kb.id) : [];
      setNotes(inKb);
      if (inKb[0]) setSelected(inKb[0].id);
    })();
  }, []);

  return (
    <div className="flex flex-1 overflow-hidden">
      <div className="w-60 border-r border-gray-200 flex flex-col">
        <input
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder="搜索知识…"
          className="m-3 border border-gray-200 rounded-md px-3 py-1.5 text-sm"
        />
        <div className="overflow-y-auto flex-1">
          {notes
            .filter((n) => !q || n.title.includes(q))
            .map((n) => (
              <button
                key={n.id}
                onClick={() => setSelected(n.id)}
                className={`w-full text-left px-4 py-2.5 border-b border-gray-100 ${
                  selected === n.id ? 'bg-blue-50' : 'hover:bg-gray-50'
                }`}
              >
                <div className="text-sm text-gray-800">{n.title}</div>
                <div className="text-xs text-gray-400">{n.created_at.slice(0, 10)}</div>
              </button>
            ))}
        </div>
      </div>
      {selected ? <NoteDetail id={selected} /> : <div className="flex-1 grid place-items-center text-gray-400">选择一条知识</div>}
    </div>
  );
}
```

`api.listCategories` 补充到 `src/lib/tauri.ts`：

```ts
listCategories: () => invoke<Category[]>('list_categories'),
```

并在 Rust `commands.rs` 增加：

```rust
#[tauri::command]
pub fn list_categories(state: State<AppState>) -> AppResult<Vec<categories_db::Category>> {
    let storage = state.storage.lock().unwrap();
    Ok(categories_db::list(storage.conn())?)
}
```

- [ ] **Step 5: 实现 MainWindow.tsx（组合侧边栏 + 内容区）**

```tsx
import { useEffect, useState } from 'react';
import { api } from '../../lib/tauri';
import { Sidebar, type SidebarSection } from './Sidebar';
import { NoteList } from './NoteList';
import { KnowledgeBase } from '../kb/KnowledgeBase';
import { ReviewPage } from '../review/ReviewPage';
import { SettingsPage } from '../settings/SettingsPage';

export function MainWindow() {
  const [active, setActive] = useState('inbox');
  const [reviewCount, setReviewCount] = useState(0);
  const [openNote, setOpenNote] = useState<string | null>(null);

  useEffect(() => {
    void api.pendingSuggestionCount().then(setReviewCount);
  }, [active]);

  const sections: SidebarSection[] = [
    { key: 'inbox', label: '今日速记', icon: '📥' },
    { key: 'review', label: '整理建议', icon: '✨', badge: reviewCount },
    { key: 'kb', label: '知识库', icon: '📚' },
    { key: 'settings', label: '设置', icon: '⚙' },
  ];

  return (
    <div className="h-screen flex">
      <Sidebar sections={sections} active={active} onSelect={(k) => { setActive(k); setOpenNote(null); }} />
      {active === 'inbox' && (
        <NoteList date={new Date().toISOString().slice(0, 10)} onOpen={setOpenNote} />
      )}
      {active === 'review' && <ReviewPage />}
      {active === 'kb' && <KnowledgeBase />}
      {active === 'settings' && <SettingsPage />}
    </div>
  );
}
```

- [ ] **Step 6: 运行测试并提交**

```bash
npx vitest run src/tests/mainwindow.test.tsx
npx tsc --noEmit
git add -A
git commit -m "feat: main window with sidebar, list, detail, knowledge base"
```

Expected: 组件测试 PASS，类型检查通过。

---

## Task 11: 整理确认页前端

**Files:**
- Create: `src/components/review/ReviewPage.tsx`
- Create: `src/components/review/SuggestionCard.tsx`
- Create: `src/tests/review.test.tsx`
- Modify: `src/lib/tauri.ts`

- [ ] **Step 1: 写组件测试**

`src/tests/review.test.tsx`：

```tsx
import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { SuggestionCard } from '../components/review/SuggestionCard';

const sug = {
  id: 1,
  note_id: '20260808-153012-ab12',
  ai_category: '待办',
  new_category_proposal: null,
  summary: '明天交周报',
  keywords: '["周报"]',
  status: 'suggested',
  created_at: '2026-08-08T20:12:00+08:00',
};

describe('SuggestionCard', () => {
  it('renders summary and actions', () => {
    render(
      <SuggestionCard
        suggestion={sug}
        original="明天下午3点交周报"
        onAccept={() => {}}
        onSkip={() => {}}
      />,
    );
    expect(screen.getByText('明天交周报')).toBeTruthy();
    expect(screen.getByText('✓ 接受')).toBeTruthy();
  });

  it('accept triggers callback', () => {
    const onAccept = vi.fn();
    render(
      <SuggestionCard suggestion={sug} original="" onAccept={onAccept} onSkip={() => {}} />,
    );
    screen.getByText('✓ 接受').click();
    expect(onAccept).toHaveBeenCalledWith(sug);
  });
});
```

- [ ] **Step 2: 实现 SuggestionCard.tsx**

```tsx
import type { Suggestion } from '../../types';

export function SuggestionCard(props: {
  suggestion: Suggestion;
  original: string;
  onAccept: (s: Suggestion) => void;
  onSkip: (s: Suggestion) => void;
}) {
  const s = props.suggestion;
  return (
    <div className="border border-gray-200 rounded-lg p-3 mb-2">
      <div className="text-xs text-gray-400">原文：{props.original}</div>
      <div className="text-sm font-medium mt-1">{s.summary}</div>
      {s.keywords ? (
        <div className="flex gap-1 mt-1">
          {(JSON.parse(s.keywords) as string[]).map((k) => (
            <span key={k} className="bg-gray-100 rounded px-1.5 text-xs text-gray-600">
              {k}
            </span>
          ))}
        </div>
      ) : null}
      {s.new_category_proposal ? (
        <div className="mt-1 text-xs text-amber-700 bg-amber-50 border border-dashed border-amber-300 rounded px-2 py-1">
          🆕 建议新分类「{s.new_category_proposal}」
        </div>
      ) : null}
      <div className="flex gap-2 mt-2">
        <button
          onClick={() => props.onAccept(s)}
          className="bg-blue-50 text-blue-700 rounded px-3 py-1 text-sm"
        >
          ✓ 接受
        </button>
        <button onClick={() => props.onSkip(s)} className="text-gray-400 px-2 py-1 text-sm">
          跳过
        </button>
      </div>
    </div>
  );
}
```

- [ ] **Step 3: 实现 ReviewPage.tsx（按分类分组 + 全部接受）**

```tsx
import { useEffect, useMemo, useState } from 'react';
import { api } from '../../lib/tauri';
import type { Suggestion } from '../../types';
import { SuggestionCard } from './SuggestionCard';

export function ReviewPage() {
  const [suggestions, setSuggestions] = useState<Suggestion[]>([]);
  const [originals, setOriginals] = useState<Record<string, string>>({});
  const today = new Date().toISOString().slice(0, 10);

  const load = async () => {
    const list = await api.listSuggestions(today);
    setSuggestions(list.filter((s) => s.status === 'suggested'));
    const map: Record<string, string> = {};
    for (const s of list) {
      map[s.note_id] = (await api.getNote(s.note_id)) ?? '';
    }
    setOriginals(map);
  };

  useEffect(() => {
    void load();
  }, []);

  const groups = useMemo(() => {
    const g = new Map<string, Suggestion[]>();
    for (const s of suggestions) {
      const key = s.new_category_proposal ?? s.ai_category ?? '其他';
      g.set(key, [...(g.get(key) ?? []), s]);
    }
    return [...g.entries()];
  }, [suggestions]);

  const accept = async (s: Suggestion) => {
    await api.acceptSuggestion(s.id, s.ai_category ?? undefined, s.summary ?? undefined, s.keywords ?? undefined);
    await load();
  };

  const skip = async (s: Suggestion) => {
    await api.skipSuggestion(s.id);
    await load();
  };

  return (
    <div className="flex-1 overflow-y-auto p-4">
      <div className="flex items-center gap-3 mb-3">
        <h2 className="text-base font-bold">{today} 整理建议</h2>
        <span className="text-xs text-gray-400">共 {suggestions.length} 条</span>
        <button
          className="ml-auto bg-blue-500 text-white rounded px-3 py-1 text-sm"
          onClick={async () => {
            await api.acceptAll(today);
            await load();
          }}
        >
          全部接受
        </button>
      </div>
      {groups.map(([cat, list]) => (
        <section key={cat} className="mb-4">
          <h3 className="text-sm text-gray-600 font-medium mb-2">{cat}（{list.length}）</h3>
          {list.map((s) => (
            <SuggestionCard
              key={s.id}
              suggestion={s}
              original={originals[s.note_id] ?? ''}
              onAccept={accept}
              onSkip={skip}
            />
          ))}
        </section>
      ))}
      {suggestions.length === 0 ? (
        <div className="text-gray-400 text-sm mt-10 text-center">今天的记录都已整理完毕 🎉</div>
      ) : null}
    </div>
  );
}
```

- [ ] **Step 4: 补充 tauri.ts 与 types.ts**

`types.ts`：

```ts
export interface Suggestion {
  id: number;
  note_id: string;
  ai_category: string | null;
  new_category_proposal: string | null;
  summary: string | null;
  keywords: string | null;
  status: 'suggested' | 'accepted' | 'adjusted' | 'skipped';
  created_at: string;
}
```

`tauri.ts`：

```ts
listSuggestions: (date: string) => invoke<Suggestion[]>('list_suggestions', { date }),
acceptSuggestion: (id: number, categoryName?: string, summary?: string, keywords?: string) =>
  invoke<void>('accept_suggestion', { suggestionId: id, categoryName: categoryName ?? null, summary: summary ?? null, keywords: keywords ?? null }),
skipSuggestion: (id: number) => invoke<void>('skip_suggestion', { suggestionId: id }),
acceptAll: (date: string) => invoke<number>('accept_all', { date }),
pendingSuggestionCount: () => invoke<number>('pending_suggestion_count'),
```

对应 Rust 命令：`list_suggestions`、`pending_suggestion_count`（`SELECT COUNT(*) FROM suggestions WHERE status='suggested'`）在 `commands.rs` 补全；`accept_all` 在 Task 8 已有。

- [ ] **Step 5: 测试 + 提交**

```bash
npx vitest run src/tests/review.test.tsx
npx tsc --noEmit
git add -A
git commit -m "feat: review page with grouped suggestions and accept/skip"
```

---

## Task 12: 设置页与首启向导（前端 + Rust）

**Files:**
- Create: `src/components/settings/SettingsPage.tsx`
- Create: `src/components/onboarding/FirstRunWizard.tsx`
- Modify: `src/lib/tauri.ts`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Rust 设置命令**

```rust
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SettingsDto {
    pub data_dir: String,
    pub hotkey: String,
    pub org_time: String,
    pub auto_org_enabled: bool,
    pub ai_base_url: String,
    pub ai_model: String,
    pub ai_api_key: String,
}

impl From<crate::config::Config> for SettingsDto {
    fn from(c: crate::config::Config) -> Self {
        Self {
            data_dir: c.data_dir.to_string_lossy().to_string(),
            hotkey: c.hotkey,
            org_time: c.org_time,
            auto_org_enabled: c.auto_org_enabled,
            ai_base_url: c.ai.base_url,
            ai_model: c.ai.model,
            ai_api_key: c.ai.api_key,
        }
    }
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> AppResult<SettingsDto> {
    let cfg = state.config.lock().unwrap().clone();
    Ok(cfg.into())
}

#[tauri::command]
pub fn update_settings(state: State<AppState>, s: SettingsDto) -> AppResult<()> {
    let mut cfg = state.config.lock().unwrap();
    cfg.hotkey = s.hotkey;
    cfg.org_time = s.org_time;
    cfg.auto_org_enabled = s.auto_org_enabled;
    cfg.ai.base_url = s.ai_base_url;
    cfg.ai.model = s.ai_model;
    cfg.ai.api_key = s.ai_api_key;
    crate::config::save(&state.data_dir, &cfg).map_err(AppError::new)?;
    Ok(())
}
```

- [ ] **Step 2: 实现 SettingsPage.tsx**

```tsx
import { useEffect, useState } from 'react';
import { api, type SettingsDto } from '../../lib/tauri';

export function SettingsPage() {
  const [s, setS] = useState<SettingsDto | null>(null);
  const [msg, setMsg] = useState('');

  useEffect(() => {
    void api.getSettings().then(setS);
  }, []);

  if (!s) return <div className="flex-1 p-6 text-gray-400">加载中…</div>;

  const save = async () => {
    await api.updateSettings(s);
    setMsg('已保存');
    setTimeout(() => setMsg(''), 2000);
  };

  return (
    <div className="flex-1 overflow-y-auto p-6 max-w-xl">
      <h2 className="text-base font-bold mb-4">设置</h2>
      <label className="block text-sm text-gray-600 mb-1">数据目录</label>
      <input
        className="w-full border border-gray-200 rounded-md px-3 py-1.5 text-sm mb-3"
        value={s.data_dir}
        readOnly
      />
      <label className="block text-sm text-gray-600 mb-1">全局快捷键</label>
      <input
        className="w-full border border-gray-200 rounded-md px-3 py-1.5 text-sm mb-3"
        value={s.hotkey}
        onChange={(e) => setS({ ...s, hotkey: e.target.value })}
      />
      <label className="block text-sm text-gray-600 mb-1">每日整理时间（HH:MM）</label>
      <input
        className="w-full border border-gray-200 rounded-md px-3 py-1.5 text-sm mb-3"
        value={s.org_time}
        onChange={(e) => setS({ ...s, org_time: e.target.value })}
      />
      <label className="block text-sm text-gray-600 mb-1">API Base URL</label>
      <input
        className="w-full border border-gray-200 rounded-md px-3 py-1.5 text-sm mb-3"
        value={s.ai_base_url}
        onChange={(e) => setS({ ...s, ai_base_url: e.target.value })}
      />
      <label className="block text-sm text-gray-600 mb-1">模型</label>
      <input
        className="w-full border border-gray-200 rounded-md px-3 py-1.5 text-sm mb-3"
        value={s.ai_model}
        onChange={(e) => setS({ ...s, ai_model: e.target.value })}
      />
      <label className="block text-sm text-gray-600 mb-1">API Key</label>
      <input
        type="password"
        className="w-full border border-gray-200 rounded-md px-3 py-1.5 text-sm mb-3"
        value={s.ai_api_key}
        onChange={(e) => setS({ ...s, ai_api_key: e.target.value })}
      />
      <label className="flex items-center gap-2 text-sm text-gray-600 mb-4">
        <input
          type="checkbox"
          checked={s.auto_org_enabled}
          onChange={(e) => setS({ ...s, auto_org_enabled: e.target.checked })}
        />
        每晚自动整理
      </label>
      <button onClick={() => void save()} className="bg-blue-500 text-white rounded px-4 py-1.5 text-sm">
        保存
      </button>
      {msg ? <span className="ml-3 text-sm text-green-600">{msg}</span> : null}
    </div>
  );
}
```

- [ ] **Step 3: 首启向导（检测 config 是否存在）**

`FirstRunWizard.tsx`：使用 `@tauri-apps/plugin-dialog` 的 `open({ directory: true })` 选择数据目录，然后调用 `init_data_dir` 命令（创建目录、写默认 config、初始化 DB），成功后刷新。

Rust 命令：

```rust
#[tauri::command]
pub fn init_data_dir(state: State<AppState>, dir: String) -> AppResult<()> {
    let path = std::path::PathBuf::from(&dir);
    std::fs::create_dir_all(&path).map_err(AppError::new)?;
    let cfg = crate::config::load(&path).map_err(AppError::new)?;
    let storage = Storage::open(&path.join("quicknote.db")).map_err(AppError::new)?;
    *state.config.lock().unwrap() = cfg;
    *state.storage.lock().unwrap() = storage;
    Ok(())
}
```

`lib.rs` 首启判断：`if !cfg.data_dir.join("config.json").exists() { 创建 first-run 窗口 }`；简化方案：主窗口打开时前端调用 `api.ensureDataDir()`，不存在则渲染向导。

- [ ] **Step 4: 注册命令、测试并提交**

```bash
npx tsc --noEmit
cd src-tauri && cargo check
git add -A
git commit -m "feat: settings page and first-run data dir wizard"
```

---

## Task 13: 窗口管理、全局快捷键与托盘（Rust）

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/default.json`

- [ ] **Step 1: 增加两个窗口定义**

`tauri.conf.json` 的 `app.windows`：

```json
[
  { "label": "main", "title": "QuickNote", "width": 1000, "height": 680, "visible": true },
  {
    "label": "capture",
    "title": "QuickNote 速记",
    "width": 420,
    "height": 360,
    "resizable": true,
    "alwaysOnTop": true,
    "skipTaskbar": true,
    "visible": false,
    "decorations": true
  }
]
```

- [ ] **Step 2: 注册快捷键、托盘与单实例**

`lib.rs` 关键片段：

```rust
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

fn setup(app: &tauri::App) {
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>).unwrap();
    let show = MenuItem::with_id(app, "show", "打开 QuickNote", true, None::<&str>).unwrap();
    let menu = Menu::with_items(app, &[&show, &quit]).unwrap();

    TrayIconBuilder::with_id("main")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => app.exit(0),
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            _ => {}
        })
        .build(app)
        .unwrap();

    let capture = app.get_webview_window("capture").unwrap();
    app.global_shortcut()
        .on_shortcut("Alt+Shift+N", move |app, _event, _ctx| {
            if capture.is_visible().unwrap_or(false) {
                let _ = capture.hide();
            } else {
                let _ = capture.show();
                let _ = capture.set_focus();
                let _ = capture.emit("capture-focus", ());
            }
        })
        .unwrap();
}
```

- [ ] **Step 3: capabilities 增加权限**

`capabilities/default.json` 追加：

```json
{
  "identifier": "default",
  "windows": ["main", "capture"],
  "permissions": [
    "core:default",
    "core:window:allow-show",
    "core:window:allow-hide",
    "core:window:allow-set-focus",
    "core:event:default",
    "global-shortcut:allow-register",
    "global-shortcut:allow-unregister",
    "dialog:default"
  ]
}
```

- [ ] **Step 4: 捕获窗 Esc 隐藏**

在 `CaptureWindow.tsx` 增加：

```tsx
useEffect(() => {
  const onKey = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      void flush();
      void window.__TAURI__.window.getCurrentWindow().hide();
    }
  };
  window.addEventListener('keydown', onKey);
  return () => window.removeEventListener('keydown', onKey);
}, [flush]);
```

（用 `@tauri-apps/api/window` 的 `getCurrentWindow().hide()` 替代 `window.__TAURI__`。）

- [ ] **Step 5: 单实例插件与 dev 验证**

`Cargo.toml` 加 `tauri-plugin-single-instance = "2"`，`lib.rs` 注册；`npm run tauri dev` 手动验证：快捷键呼出/隐藏、托盘菜单、Esc 保存隐藏。

```bash
git add -A
git commit -m "feat: tray, global shortcut, two windows, single instance"
```

---

## Task 14: 手工验收清单（E2E 冒烟测试）

**Files:**
- Create: `docs/manual-acceptance.md`

- [ ] **Step 1: 编写验收清单**

覆盖以下场景，全部通过才算 v0.1 完成：

1. 首次启动出现数据目录向导，选择 D 盘目录后 config.json/quicknote.db 生成在该目录。
2. 全局快捷键呼出捕获窗，输入中文 Markdown（标题/列表/加粗），失焦后自动保存；`.md` 出现在 `notes/YYYY/MM/`。
3. 截图后 Ctrl+V 粘贴进捕获窗，图片出现在 `attachments/YYYY/MM/<id>/`，Markdown 引用可渲染。
4. Esc 隐藏窗口；托盘"打开 QuickNote"显示主窗口。
5. 主窗口"今日速记"列出当天记录；点击可读全文。
6. 设置页填入 DeepSeek API Key，点击"立即整理"；建议出现在"整理建议"页，按分类分组。
7. 接受一条待办建议：记录出现在"待办"分类；原始 .md 未变化。
8. AI 提议新分类（用"购物清单"测试内容），接受后分类出现且可再次使用。
9. 修改数据目录并迁移：旧目录文件复制到新目录，应用重启后从新目录读取。
10. 删除 quicknote.db 后点"重建索引"，记录恢复。

- [ ] **Step 2: 逐项执行并勾选**

```bash
npm run tauri dev
```

按清单操作，勾选结果写回 `docs/manual-acceptance.md`。

- [ ] **Step 3: 提交**

```bash
git add docs/manual-acceptance.md
git commit -m "docs: manual acceptance checklist"
```

---

## Task 15: README、LICENSE 与发布推送

**Files:**
- Create: `README.md`
- Create: `LICENSE`（MIT）
- Modify: `.github/workflows/ci.yml`（可选：cargo check + tsc + vitest）

- [ ] **Step 1: 写 README（中文，含功能截图占位、架构说明、开发运行方式）**

```markdown
# QuickNote

零摩擦捕获 + AI 每日自动整理的本地优先快速记录软件。

- 全局快捷键呼出悬浮窗，输入即保存，支持 Markdown 与图片粘贴
- 记录为本地 Markdown 文件 + SQLite 索引，数据目录可自定义
- 云端 AI（OpenAI 兼容）每晚自动整理：分类、摘要、关键词，人工一键确认
- 内置分类：待办 / 进度 / 提醒 / 知识库；AI 可提议新分类（上限 10）

## 开发

```bash
npm install
npm run tauri dev
```

## 技术栈

Tauri 2 · React 19 · TypeScript · Milkdown Crepe · SQLite (rusqlite) · Tailwind CSS
```

- [ ] **Step 2: 添加 MIT LICENSE（作者 thunderzzz，年份 2026）**

- [ ] **Step 3: 推送**

```bash
git add README.md LICENSE
git commit -m "docs: readme and license"
git push -u origin main
```

Expected: GitHub 仓库 main 分支出现 v0.1 全部代码。

---

## 自审记录（执行前由计划作者完成）

1. **Spec 覆盖**：捕获窗（Task 9/13）、Markdown+图片（Task 5/9）、数据目录（Task 4/12）、SQLite+md+附件（Task 3/5）、AI 每晚整理（Task 7/8）、确认页（Task 8/11）、分类体系与上限（Task 3/8）、知识库独立页（Task 10）、重建索引（Task 6）、托盘/快捷键（Task 13）、错误处理（各任务错误类型 + Task 8 失败收集）、测试策略（每任务 TDD/组件测试 + Task 14 手工清单）。
2. **占位符**：本计划中 `renderMarkdown` 的附件路径解析与 `run_ai_org_inner` 重构已在对应任务中给出实现方向；执行时若发现缺失细节，以对应任务内代码为准补齐，不出现 "TBD"。
3. **类型一致性**：`NoteMeta`/`Suggestion`/`Category`/`SettingsDto` 前后端字段一致；`accept_suggestion` 参数名 `suggestionId` 与前端 invoke 一致；`list_notes` 的 `date` 参数与 `api.listNotes(date)` 一致。
