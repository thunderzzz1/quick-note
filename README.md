# QuickNote

零摩擦捕获 + AI 每日自动整理的本地优先快速记录软件。

## 特性

- **全局快捷键呼出**：`Alt+Shift+N`（可在设置中修改）随时弹出悬浮捕获窗，输入即保存，无需选择文件夹、创建文件
- **Markdown 所见即所得**：基于 Milkdown Crepe，支持标题、列表、加粗等富文本，剪贴板截图直接粘贴
- **本地优先**：记录为纯 Markdown 文件，图片按记录归档，数据目录可自定义（支持迁移），即使数据库损坏也能从 `.md` 重建索引
- **AI 每日整理**：每晚定时（可配置）调用 OpenAI 兼容接口（默认 DeepSeek），把当天记录分类为待办/进度/提醒/知识库等，生成摘要和关键词
- **人工确认**：AI 只给建议，你一键确认；原始记录永不被动修改；AI 可提议新分类（有效分类上限 10 个，超出归"其他"）
- **知识库**：知识类记录有独立空间，列表 + 阅读双栏，支持搜索

## 开发

前置依赖：Node.js ≥ 20、Rust stable（MSVC）、VS 2022 Build Tools（C++ 工作负载）。

```bash
npm install
npm run tauri dev
```

测试：

```bash
npm test            # 前端组件/逻辑测试
cd src-tauri && cargo test   # Rust 单元测试
```

## 技术栈

- Tauri 2（Rust 核心 + Web 前端）
- React 19 + TypeScript + Vite
- Milkdown Crepe（Markdown 所见即所得）
- SQLite（rusqlite，bundled）+ Markdown 文件存储
- Tailwind CSS

## 数据目录

首次启动时选择数据目录（建议放在非系统盘）。目录结构：

```
QuickNote/
├── config.json          # 设置（API、快捷键、整理时间）
├── quicknote.db         # SQLite 索引与元数据
├── notes/2026/08/       # Markdown 正文（按年月分层）
└── attachments/2026/08/ # 图片附件（按记录分组）
```

备份只需复制整个目录；`.md` 文件可直接用任意文本编辑器打开。

## 路线图

- [x] v0.1：捕获窗、Markdown + 图片、AI 每日整理与确认、知识库、设置
- [ ] 系统级定时提醒（当前"提醒"仅作为分类）
- [ ] 本地模型（Ollama，接口已预留）
- [ ] 多端同步 / 移动端

## License

MIT
