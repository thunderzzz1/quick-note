# QuickNote 项目协作约定

## 代码发布

- 推送/发布代码统一走 GitHub 工具链：优先使用 `github:github` / `github:yeet` 技能与 GitHub 连接器（MCP）处理仓库、Issue、PR 等操作；git 仅作为底层提交与推送工具。
- 推送时优先 GitHub 直连（本机代理 `127.0.0.1:7897` 对 git 偶发 SSL 断连）。若代理推送失败，使用：
  `git -c http.https://github.com.proxy= push origin <branch>`
- CA 证书包已配置：`C:\Users\rsl\.codex\ca-bundle.pem`（git 全局 `http.sslCAInfo`），不要回退到关闭证书校验。
- `gh` CLI 未安装；涉及 Actions 日志等需要 gh 的流程，先告知用户安装 `gh` 并登录，不要假装可用。

## 环境注意事项

- 用户级 `.npmrc` 指向已失效的淘宝镜像；安装 npm 包统一加 `--registry=https://registry.npmjs.org`。
- `testdata-tmp/` 是手工验收用的临时数据目录，已被 `.gitignore` 忽略，不要提交。
- 验收记录与问题修复记录维护在 `docs/manual-acceptance.md`。
- 设计文档在 `docs/superpowers/specs/`，实施计划在 `docs/superpowers/plans/`；`docs/` 变更按常规提交。
