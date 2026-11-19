# Core / UI / App 契约 v1

本文件是实现前的契约，不表示以下类型已经存在。基线：REQ-0004，documentRevision 1。类型名可在三项 technical_review 中统一调整，字段语义和未知状态不可丢失；变更后同步两个实现任务。

## 1. 共同约定

所有跨并发边界的 DTO 使用值类型、`Sendable`；需存储的字段采用 `Codable` 或显式 SQLite 映射。字节数用 `Int64`（非负且溢出检查），差量允许为负，时间用 UTC `Date`。ID 使用独立类型包装 UUID/稳定键，不用数组下标或显示名称代替身份。路径保存文件 URL 与原始相对路径，不能将字符串 `hasPrefix` 当作目录边界验证。

每类快照有独立 `sampleAt`、`lastSuccessAt`、`expectedInterval`、`state`、`source`。总体快照的 `publishedAt` 只表示发布时刻，不代表所有数据同时采样。

```swift
enum Availability: Sendable, Codable {
    case available
    case unknown(reason: String)
    case permissionDenied(reason: String)
    case unavailable(reason: String)
    case partial(reason: String, unreadableCount: Int)
}

// nil 必须有非 available 状态；0 只表示成功采样且确实为零。
struct Metric<Value: Sendable>: Sendable {
    let value: Value?
    let availability: Availability
}

enum Freshness: Sendable { case fresh, stale, suspended }
```

可用性、扫描阶段和新鲜度是不同维度。扫描中可继续显示“上次完整结果 + 更新时间”，不能将上次结果重新盖上当前时间。`unknown`/`partial` 不被折算成 0；排行对缺失项单列；聚合显示有效小计和不可读成员数。

## 2. 数据模型

| DTO | 必需字段/约束 |
| --- | --- |
| `SystemMemorySnapshot` | `sampleAt, lastSuccessAt, totalBytes, freeBytes, activeBytes, inactiveBytes, wiredBytes, compressedPhysicalBytes, usedEstimateBytes, usedEstimateFormula, pressure, source, availability, freshness`。各可失败指标使用 Metric。公式与平台口径见 collection-and-safety.md；不声称等同活动监视器 |
| `ProcessIdentity` | `pid, startTimeSeconds, startTimeMicroseconds, executablePath, uid`；PID 重用产生新实例。路径不可读时身份/归属分开标记，不能将 PID 单独持久化为稳定身份 |
| `AgentProcessSnapshot` | `identity, agentID?, rssBytes, firstSeen, lastSeen, sampleAt, matchEvidence[], activityEvidence[], availability`。Agent RSS 是本次可读且去重实例之和，不是系统“应用内存”；进程存在标签为“运行中”，不能直接标为“正在写文件” |
| `AgentResourceSummary` | `agentID, readableProcessIDs[], unreadableProcessCount, rssSubtotal, storageRootIDs[], observedDiskBytes, evidence[], sampleTimes`。不可确认归属单列未知，不能合并为某个 Agent |
| `MonitoredRoot` | `id, selectedURL, canonicalURL, volumeID, rootFileID, bookmarkData?, origin(autoDiscovered/userSelected), agentAssociation?, ownershipEvidence[], enabled, configuredAt`。目录内容替换/根身份改变时需复核；重叠根按身份与路径拒绝或明确合并，不能重复求和 |
| `DirectorySnapshot` | `rootID, path, sampleStart, sampleEnd, lastCompleteAt, generation, baselineAt, baselineStatus, categoryTotals, regularFileCount, logicalBytes, allocatedBytes?, symlinkCount, scanState, availability, freshness, ownershipEvidence[], coverageNotes[]`。完整总量只来自完整代次；不完整枚举不能推断删除 |
| `FileEntry` | `id, rootID, relativePath, absoluteURL, volumeID, fileID, kind, logicalBytes, allocatedBytes?, modificationTimeNs, metadataChangeTimeNs, linkCount, category, associationEvidence[], writerAttribution, lastObservedAt, safetyFlags[]`。分页返回；文件名与路径当数据处理，不执行 |
| `OwnershipEvidence` | `kind(directoryConvention/userLabel/executableIdentity/verifiedParentChain), ruleID, ruleVersion, matchedValue, scope(storageAssociation/processIdentity), confidence, observedAt, explanation`。文件写入者默认为 unknown；本版不提供写入者级审计证据 |
| `GrowthPoint` | `rootID, epochID, windowStart, windowEnd, addedCount, removedCount, addedEntryBytes, removedEntryBytes, modifiedDeltaBytes, netBytes, currentTotalBytes, coverage, continuity, source`。`net=addedEntryBytes-removedEntryBytes+modifiedDeltaBytes`，重命名同身份不计新增/删除；超出能力则说明为条目变化 |
| `ActivityEvidence` | `kind(processPresent/recentObservedFileChange/cpuDelta), observedAt, window, agentID?, confidence, reason`；文件变化只归属于监测根，不据此推断某进程的写入行为 |
| `HistoryQuery/HistoryPage` | `scope, from, to, resolution, cursor?, limit` / `points[], gaps[], nextCursor, sourceResolution`。默认最多 500 点/页；降采样不填补缺口 |
| `CleanupCandidate` | `id, rootID, fileID, relativePath, logicalBytes, category, reason, ruleID/version, ownershipEvidence[], fingerprint, stableSince, safetyFlags[], eligibility, observedAt`；只由 Core 创建，UI 不提交任意路径 |
| `CleanupPlan` | `id, candidateIDs[], items[], expectedSnapshotRevision, digest, createdAt, expiresAt, movedLogicalBytesEstimate, perVolumeBeforeObservation`；Core 暂存计划，默认 60 秒失效，元数据变化即撤销对应项 |
| `CleanupItemResult` | `candidateID, outcome(moved/skipped/failed/alreadyGone/unknownAfterInterruption), reasonCode, message, beforeFingerprint, observedAfter, resultingTrashURL?, restoreHint, startedAt, endedAt`。每项独立，失败不得覆盖其他成功结果 |
| `CleanupBatchResult` | `operationID, itemResults[], movedLogicalBytes, beforeAfterVolumeObservations[], durabilityState`；“移出监测目录”与“卷可用空间观测变化”分别显示，移入同卷废纸篓通常不增加空间 |
| `MonitorSnapshot` | `revision, publishedAt, sessionID, systemMemory, processes[], agentSummaries[], directories[], recentGrowth[], candidateSummary, persistenceState, monitoringState, warnings[]`。不包含完整文件索引和全部历史，菜单栏与主窗口从此投影 |

`ScanState` 至少表达 idle、queued、scanning(processedCount)、reconciling、ready、paused、rootMissing、failed。不知文件总数时显示已扫描数，不伪造进度百分比。`BaselineStatus` 区分首次建立、从历史恢复待复核、已确认、历史损坏后重建。

文件逻辑总量为纳入范围的普通文件条目大小之和，符号链接不跟随、不计目标。硬链接可能按路径重复，`linkCount > 1` 明示；分配字节可选且不是 APFS 唯一物理占用。应用总量要消除重叠监测根的重复统计。目录和包的展示都不能以父项大小再加子项大小。

## 3. 最小服务接口

由 Core 提供一个 façade，无需为每个 DTO 建 manager。以下是协议轮廓；错误使用稳定 code 和可读说明，权限失败通常以数据状态返回，配置/命令本身不合法才 throws。

```swift
public protocol MonitoringService: Sendable {
    func start() async
    func stop() async
    func updates() async -> AsyncStream<MonitorSnapshot>

    func discoverRoots() async -> [RootDiscovery]
    func upsertRoot(_ registration: RootRegistration) async throws -> RootID
    func setRootEnabled(_ id: RootID, enabled: Bool) async throws
    func removeRoot(_ id: RootID, retainHistory: Bool) async throws
    func requestRefresh(_ scope: RefreshScope) async

    func files(_ query: FileQuery) async throws -> FilePage
    func history(_ query: HistoryQuery) async throws -> HistoryPage
    func cleanupCandidates(_ query: CandidateQuery) async throws -> CandidatePage
    func prepareCleanup(_ ids: [CandidateID], expectedRevision: UInt64) async throws -> CleanupPlan
    func executeCleanup(_ approval: CleanupApproval) async throws -> CleanupBatchResult
    func cancelCleanupPlan(_ id: PlanID) async
}
```

- `start/stop` 幂等。AppStore 在 start 前订阅 updates；引擎立即发送初始/恢复中快照，只建立一个 AppStore 消费者。UI 重新打开不启动第二条采样循环。
- `requestRefresh` 只入队且合并同范围工作，不能等待阻塞扫描完成。配置变更用单调 revision；扫描返回结果携带 root identity、配置 revision、scan generation，过时结果丢弃。
- `RootRegistration` 由 App 完成目录选择后提交，包含 URL、可用 bookmark、标签和授权来源。用户标签只是关联证据，不升级为写入者确认。移除配置不删除用户文件；移除正在清理的根需等该批次结束或拒绝变更。
- `FileQuery/CandidateQuery` 有 root/filter/sort/cursor/limit，limit 上限 500；Core 用 SQL 排序/分页，不向 SwiftUI 返回整个树。查询结果有 revision，UI 处理翻页过期。
- `prepareCleanup` 不做文件移动，重新读取元数据后生成可展示计划；部分不合格项附原因。`CleanupApproval` 包含 planID、digest、用户确认时刻，只有 UI 的明确确认动作可调用 execute；选中候选不等于确认。
- `executeCleanup` 再次复核、逐项执行；计划缺失/过期/确认摘要不匹配直接拒绝，不接受任意 URL。命令重复以 operationID 返回已有结果，不再次移动。
- `stop` 不继续执行尚未开始的项；若外部终止导致单项结果未知，重启复核并显示 unknownAfterInterruption，不能把“原路径不存在”直接当作清理成功。

## 4. App 和 UI 的责任

App 创建 `AppStore(service:)` 和主窗口/菜单栏；AppStore 依赖上述协议。UI 实现时先用空/扫描/权限状态接入，再接真实快照；生产入口不能自动注入假数据。预览或夹具数据必须显式隔离且不冒充当前电脑。

App 负责 NSOpenPanel、bookmark 的选择/解析协调、Finder Reveal、主窗口恢复与 NSApplication 退出入口。Core 仍独立验证路径和元数据，不能信任面板返回值永远有效。若使用 security scoped URL，访问生命周期覆盖扫描/清理并成对 stopAccessing；普通非沙盒路径不假装已获得 Full Disk Access。

清理弹层展示 Core 返回的具体路径、大小、理由、失效/不合格项与废纸篓说明；确认后显示每项结果及恢复提示。菜单栏显示“监测目录磁盘总量 / 最近窗口净变化 / 运行中 Agent 数”，RAM 单列，避免没有指标名的“内存”数字。菜单栏数据不可读或过期时保留状态，不把它格式化成 0。
