# 下载、版本与发布

## 安装

从 [GitHub Releases](https://github.com/bubugx/agentLens/releases/latest) 选择对应系统的安装文件。正式版与源码标签一一对应；测试版单独标记为 Pre-release。

| 系统 | 下载文件 | 安装方式 |
| --- | --- | --- |
| macOS Apple Silicon | 文件名含 `aarch64.dmg` | 打开 DMG，将 AgentLens 拖入 Applications |
| macOS Intel | 文件名含 `x64.dmg` | 同上，选择 Intel 对应包 |
| Windows x64 | `x64-setup.exe` | 运行安装程序，需要 WebView2；安装器按需处理运行时 |
| Linux x64 / Debian、Ubuntu | `amd64.deb` | 使用系统软件安装器，或 `sudo apt install ./AgentLens_版本_amd64.deb` |
| Linux x64 / 其他发行版 | `amd64.AppImage` | 赋予执行权限后运行；桌面环境需具备 WebKitGTK 4.1 等运行依赖 |

macOS 配置的最低版本为 12.0；发布产物仍应在目标系统实测。Linux CI 基于 Ubuntu 22.04，AppImage 不保证兼容更旧的 glibc。当前 macOS 使用 ad-hoc 签名，尚未 Developer ID 签名与公证；首次打开时可按系统提示在「隐私与安全性」确认来源。Windows 尚未配置 Authenticode 签名。

同一 Release 提供 `SHA256SUMS.txt`，可用 macOS 的 `shasum -a 256 文件名`、Linux 的 `sha256sum 文件名` 或 PowerShell 的 `Get-FileHash 文件名 -Algorithm SHA256` 核对下载完整性。校验文件用于完整性核对，不替代签名。

## 维护者发布步骤

发布工作流：[release.yml](../.github/workflows/release.yml)。日常提交与 PR 先运行 [桌面检查](../.github/workflows/desktop.yml)。

1. 完成功能和必要验证，在 [CHANGELOG](../CHANGELOG.md) 添加版本章节，例如 `## 0.2.0`。
2. 运行 `npm run version:set -- 0.2.0`，同步 npm、npm 锁文件、Cargo、Cargo 锁文件和桌面配置。运行 `npm run version:check` 验证一致性。
3. 提交代码并推送到 GitHub，然后创建并推送对应标签：

   ```sh
   git tag -a v0.2.0 -m "AgentLens v0.2.0"
   git push origin main
   git push origin v0.2.0
   ```

4. 在 Actions 中检查发布工作流。它校验版本与标签、运行测试，先创建草稿，再构建四个平台目标的 DMG、EXE、DEB 和 AppImage。
5. 全部目标成功后，工作流生成 SHA-256 文件，附上介绍视频和字幕，最后公开 Release。任一构建失败，Release 保持草稿，用户不会误下载不完整的正式版。

工作流仅使用仓库内置 `GITHUB_TOKEN` 的 `contents: write`，无需个人访问令牌。仓库需允许 GitHub Actions 运行。后续配置签名证书时，使用 GitHub Secrets，不将证书、口令或私钥提交到 Git。

## 失败重试与版本管理

- 构建失败时先看对应平台日志。基础设施临时失败可重跑失败作业；已上传到草稿的同名文件会由构建动作更新。
- 修复源码需要新提交时，创建新版本标签；不要移动公开标签。预发布版本使用如 `0.2.0-beta.1` 的版本号和 `v0.2.0-beta.1` 标签。
- 也可以手动运行工作流，填入**已存在**的标签；已公开的同版本 Release 会被拒绝覆盖。
- 撤回有问题的版本时，在 GitHub 标记并说明问题，再发布修正版。程序目前不包含内置自动更新器，用户通过 Releases 下载新包安装。

## 仓库内容边界

Git 保留应用源码、npm/Cargo 锁文件、图标、测试、工作流、当前文档，以及经过筛选的公开截图和最终介绍视频。以下内容留在本机并忽略：

- `node_modules/`、`dist/`、Rust `target/`、Tauri 生成的 schema。
- SQLite 索引、WAL、真实采样报告、本机验证材料、临时录屏和 Playwright 产物。
- `.cargo/config.toml` 中的本机镜像设置、环境变量文件、Python 环境。
- `docs/video/assets/` 中可再生成的旁白、词级时间戳、界面截图与混音中间文件。

本轮旧验收报告、过期 Swift 架构草稿和非公开截图移至本机 `artifacts/repository-cleanup/`。忽略规则不会删除安装在用户数据目录中的数据库，也不会清空开发构建缓存。Git 历史中的已有提交不做重写。
