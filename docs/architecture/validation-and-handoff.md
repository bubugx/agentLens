# 验证路径、风险与交接

阶段：native-architecture。这里只确认本阶段的环境和探针结果，不替实现、性能、视觉或代码审查角色确认通过。

## 1. 本阶段执行结果

| 检查 | 结果 | 可核查证据 |
| --- | --- | --- |
| 需求读取 | bound/get 确认 REQ-0004，revision 2，documentRevision 1，ready，初次读取无讨论 | 需求空间当前绑定文档；后续交付评论会改变 revision |
| 项目约定 | 探测时项目不存在；/Users、用户目录、workspace、项目根的 AGENTS.md 均不存在 | evidence/capabilities.json |
| 系统和构建能力 | Swift 5.10 / SDK 14.4；无可用 xcodebuild | evidence/environment.json |
| 原生框架 Release 编译/链接 | 最终探针成功，2.59 秒；并非产品构建性能 | evidence/probe-results.json |
| 数据探针运行 | RSS 读取 96/96 字节；host_statistics64=0；pressure sysctl=0；SQLite open=0 | 同上；数值为探针单次采样，不是应用基线 |
| 废纸篓/文件事件 API | 符号编译链接成功，未真正移动文件/建立事件流 | 探针源码及标准输出 |
| 液态玻璃旧 SDK 负向例 | 单独 #available 编译失败：Text 无 glassEffect | evidence/glass-negative.json |
| 旧 SDK 隔离分支 | compiler + 显式 SDK feature flag 的回退 typecheck=0 | evidence/glass-fallback.json |
| XCTest | 编译失败：no such module XCTest | evidence/probe-results.json；建议独立 executable checks |
| 界面能力 | 应用枚举成功、截图收到 PNG；AX snapshot 缺权限 | evidence/capabilities.json；未保存可能含其他应用内容的截图 |
| 目录发现 | 仅存在性探测；Codex/Claude 根存在，子目录存在性有明确结果 | evidence/discovery.json；不等于扫描与归因已实现 |

最初探针遇到 Swift C 宏和指针导入差异，修正后才形成成功结果：`PROC_PIDPATHINFO_MAXSIZE` 宏不能直接导入 Swift，可用 `4 * Int(MAXPATHLEN)` 或由 C shim 暴露；`trashItem` 函数指针使用 `AutoreleasingUnsafeMutablePointer<NSURL?>?`。初次失败结果保存在 evidence/probe-initial-results.json，避免后续实现重复踩坑。CSQLite 只需系统 header/link shim，移除 pkgConfig 能避免本机缺失 pkg-config 的非必要依赖。

Lumo terminal 在当前会话目录运行环境核验成功，但拒绝把 cwd 设为工作区外的临时目录。本阶段未创建连接或扩大该工具授权；隔离探针通过会话已有的 unrestricted `exec_command` 在用户指定的临时目录执行。后续需持续观察的应用仍应优先使用宿主提供的 terminal/computer 能力，并按实际项目引用和权限执行。

## 2. 按需求的后续验收清单

| 需求 | 必须验证的行为与判据 | 角色/证据 |
| --- | --- | --- |
| req-1 | Package.swift 与源码存在；Release 构建、.app 组装、codesign verify、实际打开；记录真实 bundle、SDK、最低系统和 CPU 范围 | integrate-app 提供日志及 dist/AgentLens.app；独立者复核 |
| req-2 | 主窗口/菜单栏的明暗主题；减少透明度和动态效果；窄窗口、长路径、键盘焦点；macOS 26 真液态玻璃与 14 回退分别验收 | native-design 引用 + UI 对应节点说明；真实截图/操作记录；缺系统明确列未验证 |
| req-3 | 已知大小夹具与真实系统/可识别进程；RAM 与磁盘分区；RSS 成员去重、PID 重用、进程消失与拒绝；未知不能显示 0 | CoreChecks + 独立交互 + code-review |
| req-4 | 已存在/新增/重叠/含链接/不可读根；实际路径、规则、用户标签、未知写入者；首扫不记新增 | 隔离夹具，逐项前后快照；不可只匹配源码字符串 |
| req-5 | 文件创建/删除/改大/重命名；partial 不产生假删除；事件丢失重扫；持续增长窗口；进程重启、应用重启、睡眠、坏库恢复与历史缺口 | 同一真实目录和持久化库跨进程验证；时间戳与 DB/界面证据 |
| req-6 | 最小化、关闭后继续采样；菜单展开、恢复主窗口、显示数据一致；明确退出后无采样/文件事件/残留进程 | 真实 App 操作、进程/日志时间线，AX 权限补齐后执行或人工记录 |
| req-7 | output/settings/sessionData 默认保留；安全候选确认后进废纸篓；符号链接/祖先替换/变化/仍写/不可读/消失/部分失败/不支持废纸篓逐项结果；原路径和垃圾桶身份复核；无永久删除回退 | 仅隔离夹具允许执行真实移动；检查恢复、journal、移出逻辑大小与卷可用空间分开 |
| req-8 | RAM/进程 ≤5 秒；稳定文件变化 ≤10 秒；固定 1 万文件首扫 ≤30 秒；Release 空闲 5 分钟平均 CPU <单核 1%，RSS ≤150 MiB | 性能角色记录硬件/SDK/系统、夹具生成器、缓存状态、采样周期、结果分布和原始日志 |
| req-9 | 最新需求版本/关联/讨论、构建授权监测清理限制文档与实际一致；独立最终复核 | 需求空间 delivery/review 记录与真实产物链接 |

## 3. 建议的行为夹具与测量方法

这些是测试设计，不是本阶段已运行结果。

- 单个隔离根由 harness 创建，并记录 seed、普通文件数、大小分布、目录层级、卷/文件系统；性能夹具固定 10,000 个普通文件，可增加独立符号链接/硬链接/权限场景，但不能把它们混入首扫可比数据集而不说明。
- 基线后顺序执行增 3、删 2、改大 1、同卷重命名 1；检查观测计数、修改差量与净变化公式，非纯类型/实现镜像断言。事件在稳定之后的单调时钟起点到 Core 快照和界面可见各计一次延迟。
- 重启恢复用实际应用/检查程序退出并新进程读取同库验证，不能仅把内存对象 encode/decode 当作重启通过。坏库、只读库、未来 schema 与磁盘写失败分别验证数据缺口和清理阻断。
- 权限夹具要核验执行 UID 和实际 EACCES，chmod 后仍可读不能算成功模拟。真实 TCC 不可伪造；注入 sampler 错误可以测 UI 分支，但需要与真实权限证据分开记录。
- 清理测试只对 harness 自建文件运行 trashItem；候选确认后继续写入、替换末端/祖先为符号链接、文件消失、trash adapter 报错、单项成功后进程终止，验证 no-follow 与不重放。禁止拿真实 ~/.codex、~/.claude 或用户产出进行验收清理。
- CPU 定义为 `Δ(进程 user+system CPU seconds)/Δwall seconds ×100`，100% 表示占满一个逻辑核。首扫完成后，在规定的无变更监测目录、菜单栏常驻/主窗口关闭状态观察 300 秒，每秒取样；报告平均 CPU、RSS 最大值及分布，包含所有 helper（若实现引入）。另做主窗口显示时对照，不能只用启动瞬时或系统平均值代替。
- 首扫时记录冷/暖缓存条件；macOS 不一定能在无特权条件下可靠清空缓存，应明确“未强制冷缓存”，固定数据集重复测量并报告分布。不为了通过放宽 30 秒、5/10 秒或资源阈值。

## 4. 风险与下一步

| 优先级 | 风险/缺口 | 处理与责任 |
| --- | --- | --- |
| 高 | 本机无 macOS 26 / SDK 26，无法证明真液态玻璃分支编译与渲染通过 | UI 实现保留明确双条件；协调者安排受支持环境验收，最终交付列实际覆盖范围 |
| 高 | trashItem 的路径 TOCTOU；权限/活跃性无法证明绝对安全 | technical-core 与 code-review 审核逐项 no-follow/身份复核、活动否决与竞态夹具；不满足则禁用对应清理项并记录 |
| 高 | 应用范围很大时扫描和完整索引可能超出 RAM/CPU 预算 | SQLite 分批索引、单扫描 worker、事件合并、分页和压力测量；超标后优化，不能悄悄截断监测范围 |
| 高 | 辅助功能未授权，真实点击/填写与控件读取受阻 | 后续测试只补齐 Lumo AX 权限或由独立者提供可核查人工操作，不能将截图或构建当作交互通过 |
| 中 | 本机缺 XCTest，若建立 testTarget 将阻塞检查 | builder 用 executableTarget 行为 harness；后续 Xcode 支持为可选增强，不降低断言 |
| 中 | 目录关联无法证明文件作者；采样可能漏掉瞬时创建后删除 | 每条证据标范围，UI 用“监测目录变化/存储关联”；未知写入者和采样限制明确展示 |
| 中 | 进程权限/解释器运行规则导致 RSS 部分可用 | 明示可读小计与未知成员；不把 node/python 整体算作 Agent |
| 中 | 移到废纸篓不等于释放卷空间；APFS/硬链接影响分配字节 | 独立命名逻辑大小、分配大小、卷可用空间；结果仅报告观测 |
| 中 | 大库、迁移、崩溃与历史丢失 | transaction/staging generation、schema guard、损坏保留、journal 与重启检查 |
| 低 | Intel、macOS 14.0/15、未来系统没有真实运行覆盖 | 不扩大“已验证兼容”声明；按需补交叉构建及设备验证 |

交给 technical-architect：合并设计引用后核对组件状态、布局与材质适配；交给 technical-core：确认系统采集、目录/进程规则、SQLite 代次和清理复核；交给 technical-ui：确认单 AppStore、生命周期和所有状态的展示。三项评审完成前不开始本阶段以外的实现。对缺口的安排由 product-strategist 统筹，本阶段不自行分配人员或宣布全团队完成。
