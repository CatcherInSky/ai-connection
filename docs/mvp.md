# AI Connection Monitor - MVP 文档

## 1. 目标与范围

### 目标
提供一个跨平台桌面常驻小工具（Windows / Linux / macOS），周期性检测到多个 AI 服务（如 Claude、Cursor、Gemini 等）的：
- **连通性**：能否成功访问；
- **应用层延迟**：HTTP 请求 RTT。

不关心中间经过直连还是代理，只关注"最终使用当前系统网络配置时，这些服务是否可用、时延如何"。
记录历史结果，并用图表直观展示最近一天（可扩展到多天）的网络质量变化。

### 暂不实现
- 不做带宽测速（下载/上传速度）。
- 不控制或识别具体代理工具（v2rayN、Hysteria2 等），它们只作为系统路由的一部分存在。
- 不做账号地区/封禁绕过，只观察网络表现。

---

## 2. 技术栈与运行形态

- **桌面应用壳**：Tauri，目标平台 Windows / Linux / macOS
- **前端**：原生 HTML/JS（当前实现）；可演进为 TypeScript + React
- **后端核心**：Rust（Tauri 后端），负责定时调度、HTTP 探测、SQLite 持久化、事件推送

### 2.1 打包与分发
详见 [packaging.md](./packaging.md)。

- **开发**：`cargo tauri dev`
- **打包**：`cargo tauri build`（需在目标平台上执行）
- **输出**：Windows (.msi/.exe)、macOS (.app/.dmg)、Linux (.AppImage/.deb/.rpm)

### 2.2 托盘常驻
应用支持系统托盘常驻，关闭主窗口时最小化到托盘而非退出。

**托盘菜单选项**：
| 选项 | 功能 |
|------|------|
| 显示主窗口 | 显示并聚焦主窗口 |
| 立即探测全部 | 立即对所有启用服务执行一轮探测 |
| 退出 | 完全退出应用 |

**关闭行为**：点击窗口关闭按钮 → 隐藏到托盘，进程继续运行、后台探测照常进行。

---

## 3. 核心实体模型

### Service（AI 服务）
| 字段 | 说明 |
|------|------|
| id | string（如 "claude", "cursor", "gemini"） |
| name | string（展示名） |
| baseUrl | string，**必须使用正确官方域名**（见 8.2） |
| probePath | string（如 `/` 或 `/v1/models`） |
| timeoutMs | number（例如 2000–5000） |
| enabled | boolean |

### ProbeResult（探测结果）
| 字段 | 说明 |
|------|------|
| timestamp | number（Unix ms） |
| serviceId | string |
| reachable | boolean |
| statusCode | number \| null |
| latencyMs | number \| null |
| errorType | "timeout" \| "dns" \| "tls" \| "http" \| "network" \| "unknown" \| null |
| estimatedBytes | number |

### Settings（全局设置）
| 字段 | 说明 |
|------|------|
| probeIntervalMs | number（默认 30000ms） |
| dailyTrafficBudgetKB | number（默认 50000KB ≈ 50MB） |
| services | Service[] |

---

## 4. 探测逻辑

### 4.1 单次探测流程
1. 构造 URL：`service.baseUrl + service.probePath`
2. 记录开始时间 t0
3. 使用 Rust HTTP 客户端发起 **HTTPS HEAD 请求**（或按需降级为轻量 GET）
4. 超时设为 `service.timeoutMs`
5. 收到响应头或发生错误时记录 t1，`latencyMs = t1 - t0`
6. **reachable 判定**：收到任意 HTTP 响应（含 401、404）即视为可达；仅连接失败、超时、DNS 失败等为不可达
7. 将结果写入 SQLite 并推送到前端（`probe_result` 事件）

### 4.2 调度器（含修复需求）

**当前实现问题**：
- 探测为**串行**执行，未并发
- 首次探测需等待一个完整 `probeIntervalMs` 周期
- "立即探测"仅针对**所选服务**，而非全部

**目标行为**：
- **启动即探测**：应用打开后立刻对全部启用服务执行一轮探测，不等待第一个间隔
- **全服务同时探测**：每个周期内对所有启用服务**并发**探测（`tokio::join!` 或 `futures::join_all`）
- **立即探测全部**：点击"立即探测"应对所有启用服务并发探测，不依赖服务选择下拉框

**建议参数**：
- `probeIntervalMs`：轮询间隔
- `maxConcurrentProbes`：同一时刻最大并发探测数（如 5，通常服务数≤5 即可全并发）

### 4.3 流量控制（估算级）
- 按天统计 `ProbeResult.estimatedBytes` 之和
- 接近预算时自动放慢间隔或仅提醒
- 超过预算时暂停自动探测，仅允许手动探测

---

## 5. 数据存储与历史可视化

### 5.1 SQLite 结构
表 `probes`：id, timestamp, service_id, reachable, status_code, latency_ms, error_type, estimated_bytes

### 5.2 后端接口
- `get_recent_probes(serviceId, sinceMs)` → ProbeResult[]
- 可选：`get_recent_probes_all(sinceMs)` → Map<serviceId, ProbeResult[]>，用于多服务图表

---

## 6. UI 设计（含修复需求）

### 6.1 主窗口

**顶部状态栏**
- 当前探测间隔
- 今日估算流量
- **可达服务：X/Y**（无需依赖服务选择）

**左侧：当前状态表**
- 每个服务一行：服务名、状态（可达/失败）、延迟(ms)、HTTP 状态码、错误类型
- 所有服务同时展示，无需选择

**右侧：历史图表**
- **必须显示坐标轴**：X 轴时间（如 "00:00"、"12:00"、"24:00"），Y 轴延迟（如 "0"、"500ms"、"1000ms"、"2000ms"）
- 折线图：X=时间，Y=延迟(ms)
- 不可达点用红色标记或断线显示
- **当前问题**：Canvas 仅绘制折线和边框，无刻度、无标签，用户无法解读具体数值

**控件**
- 服务选择下拉：**仅用于切换图表显示哪个服务的历史**，与探测逻辑解耦
- 探测间隔输入 + 保存设置
- **“立即探测”**：应对**全部**启用服务并发探测，而非当前所选服务

### 6.2 图表坐标轴实现要点
- X 轴：时间标签，如 `HH:mm` 或 `MM-DD HH:mm`，等间隔采样
- Y 轴：延迟(ms) 刻度，如 0、500、1000、2000、5000，带单位
- 使用 `ctx.fillText` 绘制文字，或引入轻量图表库（如 Chart.js）确保可读性

---

## 7. 错误分类与日志
- DNS 失败 → `"dns"`
- TCP/TLS 建连失败 → `"network"` 或 `"tls"`
- 超时 → `"timeout"`
- HTTP 非 2xx/3xx → `"http"`（本工具主要用于连通性判断，401/404 仍视为可达）
- 其他 → `"unknown"`

---

## 8. 已知问题与修复计划

### 8.1 HTTP 404 的含义（非 Bug）

**现象**：Claude、Gemini 等显示 HTTP 404。

**原因**：AI 服务 API 的根路径（`/`）通常没有资源，HEAD/GET 请求会返回 404。  
本工具的目标是**连通性检测**，而非校验 API 语义正确性。

**结论**：
- 收到 404 = 已建立 TCP/TLS 连接并收到 HTTP 响应 → **reachable = true** 是正确的
- 404 仅表示“路径不存在”，不表示“服务不可达”
- 若需 200 响应，需使用带鉴权的正式端点（如 `/v1/models`），但会增加复杂度；当前方案在无 API Key 下即可完成连通性测试，符合 MVP 目标

### 8.2 Cursor 超时：域名错误

**现象**：Cursor 服务始终 timeout。

**原因**：当前配置使用 `https://api.cursor.sh/`。  
根据 [Cursor 官方文档](https://cursor.com/docs/api)，对外 API 域名为 **`https://api.cursor.com/`**。  
`api.cursor.sh` 可能为内部或旧版域名，非公开可访问或网络策略不同，导致超时。

**修复**：将 Cursor 的 `baseUrl` 改为 `https://api.cursor.com/`。

### 8.3 服务配置速查（正确域名）

| 服务 | 正确 baseUrl | 说明 |
|------|--------------|------|
| Claude | `https://api.anthropic.com/` | 根路径通常 404，可达性判断以收到响应为准 |
| Cursor | `https://api.cursor.com/` | **勿用 api.cursor.sh** |
| Gemini | `https://generativelanguage.googleapis.com/` | 根路径可能 404，同上 |

### 8.4 修复项汇总

| 优先级 | 问题 | 修复方向 | 状态 |
|--------|------|----------|------|
| P0 | Cursor 一直 timeout | 将 baseUrl 改为 `https://api.cursor.com/` | ✅ 已修复 |
| P0 | 图表无坐标轴 | 为 Canvas 或图表组件添加 X/Y 轴刻度和标签 | ✅ 已实现 |
| P1 | 仅选中的服务被探测 | “立即探测”改为并发探测全部启用服务 | ✅ 已实现 |
| P1 | 启动后等一周期才探测 | 调度器启动时立即执行一轮全服务探测 | ⚠️ 仍有 3s 延迟 |
| P1 | 探测串行执行 | 使用 `tokio::join!` 等实现并发探测 | ✅ 已实现 |
| P2 | 服务选择与探测绑定 | 服务选择仅影响图表展示，探测始终针对全部服务 | ✅ 已解耦 |

---

## 9. 待办（按优先级）

- [x] **P0** 修正 Cursor `baseUrl` 为 `https://api.cursor.com/`
- [x] **P0** 为延迟图表添加 X 轴（时间）和 Y 轴（延迟 ms）刻度与标签
- [x] **P1** “立即探测”改为探测全部启用服务
- [ ] **P1** 应用启动时立即执行一轮全服务探测（当前有 3 秒延迟）
- [x] **P1** 探测调度改为并发执行
- [x] **P2** 明确服务选择仅用于图表，与探测逻辑解耦
- [x] **新增** 托盘常驻、关闭时隐藏到托盘、托盘菜单（显示/立即探测/退出）
- [x] **新增** 图表缩放功能（时间轴 0.5×～16×）
- [x] **新增** 打包文档（见 packaging.md）
