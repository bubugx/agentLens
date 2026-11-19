# AgentLens 介绍视频

[观看最终 MP4](../media/agentlens-intro-zh.mp4) · [中文字幕](../media/agentlens-intro-zh.srt) · [旁白](narration.md) · [分镜配置](storyboard.json)

本版用同一空间里的资源碎片、光学镜头和归属轨道串联故事。碎片从散落状态穿过扫描面，汇聚成工具轨道，再按持续观察与文件保护规则分流；经过核验的候选进入垃圾桶轨道。真实应用操作画面嵌入同一场景，避免用章节卡片轮播。

内容覆盖：为什么需要工具、资源来源追踪、进程自动观测、无 CLI 桌面应用接入、清理边界、操作复核和本地历史。旁白不包含 Vue、Rust、Tauri 等技术方案。所有界面均为应用真实浏览器预览，使用明确标记的示例数据，不承诺特定节省量。

- 1920 × 1080、30 fps、H.264 / AAC，约 68.3 秒。
- 中文 Yunxi 旁白，语速 `+60%`，约正常语速的 1.6 倍，配有按语音时间戳对齐的内嵌字幕及 SRT / WebVTT。
- 程序生成的原创电子配乐；没有使用外部版权音乐素材。
- 最终视频、封面、字幕纳入公开仓库并随 Release 提供；中间音频、截图和录屏放在忽略的 `assets/`。

## 重建

制作依赖独立于应用运行依赖，使用 Python 3.12+。从项目根目录执行：

```sh
python3 -m venv .venv-video
.venv-video/bin/python -m pip install -r docs/video/requirements.txt
npm ci
npm run dev
```

保持开发服务器运行，另开终端：

```sh
node docs/video/capture.mjs
node docs/video/capture_motion.mjs
.venv-video/bin/python docs/video/generate_voice.py
.venv-video/bin/python docs/video/prepare_motion.py
.venv-video/bin/python docs/video/render_video.py --preview
.venv-video/bin/python docs/video/render_video.py
.venv-video/bin/python docs/video/verify_video.py
```

语音生成调用 Edge TTS，需要网络；生成后可以离线复用素材。界面采集使用本机 Google Chrome，并且不会进行真实文件清理或真实进程终止。Windows 的 Python 路径可替换为 `.venv-video\Scripts\python.exe`。

默认字体使用 macOS PingFang SC。其他系统请设置 `AGENTLENS_CN_FONT` 为中文字体路径，并按字体实际情况调整 TTC 字面索引；FFmpeg 由 imageio-ffmpeg 提供，也可使用 `FFMPEG_BINARY` 指定。

`--preview` 生成本地分镜总览和关键帧，完整渲染检查时长小于 80 秒。`verify_video.py` 检查视频流、全片解码、字幕顺序与时间边界、音频响度、本地文档链接和旁白技术栈词汇，将结果写入忽略的 `validation.json`。人工检查还需观看连续运动与语音节奏，不能仅凭自动检查判断创意效果。
