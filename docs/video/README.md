# AgentLens 中文介绍视频

成片：[agentlens-intro-zh.mp4](../media/agentlens-intro-zh.mp4)。封面：[agentlens-poster.jpg](../media/agentlens-poster.jpg)。启动 `npm run dev` 后，可打开 [视频预览页](http://127.0.0.1:1421/docs/video/index.html)。

## 交付规格

| 项目 | 规格 |
| --- | --- |
| 时长 | 76.2 秒 |
| 画面 | 1920 × 1080，16:9，30 fps |
| 视频格式 | MP4，H.264，yuv420p，faststart |
| 音频 | 中文合成男声，AAC 立体声，48 kHz |
| 语速 | `zh-CN-YunxiNeural`，`rate=+60%`，约正常语速的 1.6 倍 |
| 字幕 | 33 段中文字幕，已烧录；另附可编辑 SRT |
| 配乐 | 程序合成的电子氛围、琶音、低频脉冲和转场声，无第三方音乐采样 |
| 素材 | 项目自身 Logo、浏览器预览实际界面、程序绘制图形 |

## 叙事与视觉

影片用「资源消耗分散」切入，依次回答为什么做、工具是什么、如何使用、如何实现和有什么价值。视觉采用深海军蓝、低饱和青色、高对比字体、细网格、流动光轨、镜头式同心圆、界面轻推近、快速交叉转场与数据管线动画。

| 时间 | 主题 | 画面内容 |
| --- | --- | --- |
| 00:00–00:08 | 为什么需要它 | 缓存 / 会话、内存、后台任务不断累积 |
| 00:08–00:16 | AgentLens 是什么 | Logo、工具定位、六类工具环绕 |
| 00:16–00:25 | 资源总览 | 磁盘、RSS、CPU、趋势、菜单栏速览的价值 |
| 00:25–00:34 | 追踪来源 | 实际存储界面、自动发现、目录下钻与文件原因 |
| 00:34–00:44 | 理解进程 | 实际进程解释界面、用途、身份与停止前检查 |
| 00:44–00:53 | 如何实现 | Vue → Tauri → Rust → SQLite 动态管线 |
| 00:53–01:02 | 如何清理 | 保留期、稳定观测、确认复核与系统垃圾桶 |
| 01:02–01:12 | 本地优先与收益 | 有限元数据、无监测数据上传、资源可见与操作可复核 |
| 01:12–01:16 | 品牌落版 | AgentLens、核心价值与本地启动命令 |

时间表按秒取整；精确分镜时间见 [video-metadata.json](../media/video-metadata.json)。完整旁白见 [narration.md](narration.md)，字幕见 [agentlens-intro-zh.srt](../media/agentlens-intro-zh.srt)。

## 内容依据

功能描述来自当前源码与 [实现说明](../implementation.md)。界面素材由 `capture.mjs` 打开本地浏览器预览采集；预览数据的数值不代表实测收益。影片在界面镜头中持续标明「实际界面 · 示例数据」，并说明「清空系统垃圾桶后，才真正释放磁盘空间」。

影片不承诺清理收益、性能提升比例或完整文件归因。工具的实际作用是帮助查看、追踪、检查和复核。配音通过 Microsoft Edge TTS 合成，发送的内容仅为公开介绍文案；使用已保存的音频重新渲染时无需网络。

## 文件与重建

- `storyboard.json`：分镜标题、旁白、配音设置、音频时长。
- `capture.mjs`：采集实际预览界面，使用项目现有 Playwright。
- `generate_voice.py`：从旁白生成各分镜 MP3 和词级时间戳，需要网络。
- `render_video.py`：绘制动画、按词级时间戳生成字幕、合成配乐并编码视频。
- `verify_video.py`：检查流信息、全片解码、音频响度、字幕时序和文档链接，保存核验记录。
- `assets/`：实际界面截图、各镜头音频与字幕时间戳；已包含，便于离线复用。
- `../media/`：MP4、SRT、封面、分镜总览和关键帧。

制作脚本使用 **Python 3.12+**，视频工具依赖与应用本身的 npm / Cargo 依赖独立。在项目根目录执行：

```sh
python3 -m venv .venv-video
.venv-video/bin/python -m pip install -r docs/video/requirements.txt

# 使用现有素材，先快速生成封面、关键帧和分镜总览
.venv-video/bin/python docs/video/render_video.py --preview

# 渲染完整成片（配音素材已包含，无需重新联网生成）
.venv-video/bin/python docs/video/render_video.py
```

Windows 可用 `.venv-video\Scripts\python.exe` 替换 Python 路径。默认字体来自 macOS；其他系统需通过环境变量指定支持中文的字体文件：`AGENTLENS_CN_FONT`、`AGENTLENS_EN_FONT`、`AGENTLENS_MONO_FONT`。中文 TTC 字体默认使用 PingFang SC 对应的字面索引；如改用其他字体，需同步调整脚本的字体索引。FFmpeg 由 `imageio-ffmpeg` 提供，也可设置 `FFMPEG_BINARY`。

需要更新素材时：

```sh
# 终端 1
npm run dev

# 终端 2：重新采集界面
node docs/video/capture.mjs

# 修改 storyboard.json 中的旁白后，重新合成语音与词级时间戳
.venv-video/bin/python docs/video/generate_voice.py

# 重新渲染；脚本会检查最终时长小于 80 秒
.venv-video/bin/python docs/video/render_video.py
.venv-video/bin/python docs/video/verify_video.py
```

可视化检查使用 [分镜总览](../media/agentlens-contact-sheet.jpg) 与 `frame-*.jpg`。成片完成后还应检查视频 / 音频流、全片解码、字幕时序、峰值与时长；本次记录见 [validation.json](validation.json)。
