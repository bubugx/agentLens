# 参与贡献

欢迎改进识别规则、平台兼容、用途解释、交互、多语言与文档。项目当前尚未声明许可证；提交或再分发前请留意仓库的许可证状态。

使用 Node.js 22+、npm 和 Rust stable（至少 1.88），并安装对应操作系统的 Tauri 2 系统依赖。依赖只使用 `package-lock.json` 和 `src-tauri/Cargo.lock` 管理。

```sh
npm ci
npm run desktop
npm run format:check
npm run build
npm test
cargo test --manifest-path src-tauri/Cargo.toml --lib
npm run test:e2e
```

Playwright 默认使用本机 Google Chrome。界面测试与素材采集均使用标记清楚的预览数据；真实进程停止和文件清理应先在隔离目录中验证。测试截图写入忽略的 `artifacts/`，不要覆盖 README 的公开截图。

调整清理或建议停止规则时，请说明匹配证据、受保护数据、运行中行为和执行前复核，并增加能验证边界条件的回归用例。不要通过放松保护来增加候选数量。

提交 Issue 时提供系统、应用版本、复现步骤、预期与实际结果。请隐去本机用户名、项目路径、命令参数和会话内容；不要上传 SQLite 索引、凭据或原始聊天文件。发布流程见 [版本与发布](docs/releases.md)。
