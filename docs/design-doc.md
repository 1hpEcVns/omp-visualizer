# OMP Session Visualizer — 系统设计文档

## 1. 需求分析

### 1.1 项目背景
OMP (Oh My Pi) 是一款 AI 编程助手（coding agent），其会话数据以 JSONL 格式存储在 `~/.omp/agent/sessions/` 目录下。开发者需要一种可视化工具来浏览、检索和分析这些会话记录，类似于 [minelogue](https://github.com/WeZZard/minelogue) 为 Claude Code 提供的可视化界面。

### 1.2 用户故事

| Epic | Story | 描述 |
|------|-------|------|
| 会话浏览 | S1 | 作为开发者，我希望查看所有历史会话列表，以便快速定位感兴趣的会话 |
| 会话浏览 | S2 | 作为开发者，我希望按标题、目录或会话ID搜索会话 |
| 时间线视图 | S3 | 作为开发者，我希望以时间线（Timeline）形式查看会话中的所有消息和事件 |
| 时间线视图 | S4 | 作为开发者，我希望查看每条消息的具体内容（文本、工具调用、思考过程） |
| 瀑布流视图 | S5 | 作为开发者，我希望以瀑布流（Waterfall）形式连续阅读消息内容 |
| 搜索 | S6 | 作为开发者，我希望在会话内容中搜索关键词并定位到具体消息 |
| 子代理 | S7 | 作为开发者，我希望查看子代理（subagent）的会话记录 |

## 2. 系统架构

```
┌─────────────┐     ┌──────────────────┐     ┌──────────────────┐
│   Browser   │────▶│  Rust Backend    │────▶│  OMP Sessions    │
│ (minelogue  │     │  (Axum + Tokio)  │     │  (~/.omp/agent/  │
│  frontend)  │◀────│                  │◀────│   sessions/)     │
└─────────────┘     └──────────────────┘     └──────────────────┘
      │                      │
      │  /api/conversation/  │  JSONL Parser
      │  omp/{id}/timeline   │  Session Store
      │  /api/sessions       │  Capsule Seeds
      │  /static/*           │  Template Engine (Tera)
      │                      │
      ▼                      ▼
┌─────────────┐     ┌──────────────────┐
│  Frontend    │     │  Nix Flake       │
│  (HTML/CSS/  │     │  (Reproducible   │
│   JS from    │     │   Build)         │
│   minelogue) │     └──────────────────┘
└─────────────┘
```

### 2.1 技术选型

| 组件 | 技术 | 理由 |
|------|------|------|
| 后端语言 | Rust | 类型安全、高性能JSON解析、零成本抽象 |
| Web框架 | Axum 0.8 | 基于Tokio的异步框架，Tower中间件生态 |
| 模板引擎 | Tera | 类Jinja2语法，与minelogue的Jinja2模板兼容 |
| 序列化 | Serde | 编译时零成本序列化/反序列化 |
| 前端 | minelogue前端（重用） | 成熟的会话可视化UI，支持Timeline和Waterfall布局 |
| 构建系统 | Nix Flakes | 可复现构建，依赖锁定 |
| 部署 | 单二进制部署 | 开发阶段简化部署 |

## 3. 业务流程图

### 3.1 会话加载流程

```
用户打开浏览器
    │
    ▼
GET /conversation/omp/{session_id}
    │
    ▼
服务端渲染 conversation.html
(设置 data-agent="omp", data-session-id)
    │
    ▼
前端 JS 加载完成
    │
    ▼
GET /api/conversation/omp/{id}/timeline
    │
    ▼
后端解析 JSONL 文件
    ├── 读取 session header
    ├── 逐行解析 SessionEntry
    └── 构建 capsule_seeds
    │
    ▼
返回 Protocol v2 Timeline Payload
    │
    ▼
前端渲染时间线视图
    ├── 显示消息胶囊
    ├── 显示事件标记
    └── 支持搜索和过滤
```

### 3.2 Timeline 渲染流程

```
Timeline Boot → collectTranscripts()
    │
    ├── 构建 track models
    │   ├── main track (agentPath: "main")
    │   └── subagent tracks
    │
    ├── buildCapsulesFromSeeds()
    │   ├── 解析 capsule_seeds 紧凑格式
    │   ├── 构建 capsule objects
    │   └── 注册 nav address
    │
    └── buildModels()
        ├── 注册 raw events
        ├── 注册 problem flags
        └── 建立父子关系
```

## 4. 数据库字典

本系统目前采用无数据库架构，直接从 JSONL 文件读取数据。会话元数据缓存在内存中。

### 4.1 JSONL 文件结构

| 字段 | 类型 | 说明 |
|------|------|------|
| type | string | 条目类型（session/message/compaction等） |
| id | string | 8字符十六进制ID |
| parentId | string? | 父条目ID，null表示根条目 |
| timestamp | string | ISO 8601时间戳 |
| message.role | string | 消息角色（user/assistant/developer/toolResult） |
| message.content | array | 内容块数组（text/thinking/toolCall/tool_result等） |

### 4.2 API 响应模型

#### ConversationSummary
| 字段 | 类型 | 说明 |
|------|------|------|
| id | string | 会话ID |
| title | string? | 会话标题 |
| directory | string? | 工作目录 |
| messageCount | int | 消息数量 |
| subagentCount | int | 子代理数量 |
| time_created | int? | 创建时间（Unix毫秒） |

#### Capsule Seed（紧凑格式）
| 字段 | 类型 | 说明 |
|------|------|------|
| k | string | 类型："m"=消息, "r"=原始事件 |
| ln | int | JSONL行号 |
| ei | int | 事件索引 |
| mid | string? | 消息ID |
| role | string? | 消息角色 |
| parts | array? | 内容块描述符 |
| pv | string | 预览文本（140字符） |
| ts | int? | 时间戳 |

## 5. PRD Epic-Story 结构

### Epic 1: Session Browsing（会话浏览）
- **Story 1.1**: 会话列表页面 — 显示所有历史会话，支持搜索和过滤
- **Story 1.2**: 会话详情页 — 点击会话进入时间线视图
- **Story 1.3**: 子代理浏览 — 查看子代理的会话记录

### Epic 2: Timeline View（时间线视图）
- **Story 2.1**: 时间线渲染 — 将JSONL条目渲染为可视化时间线
- **Story 2.2**: 消息详情 — 点击胶囊查看完整消息内容
- **Story 2.3**: 瀑布流视图 — 连续阅读模式
- **Story 2.4**: 布局切换 — 在Timeline和Waterfall之间切换

### Epic 3: Search（搜索）
- **Story 3.1**: 全文搜索 — 在会话内容中搜索关键词
- **Story 3.2**: 搜索结果定位 — 点击搜索结果跳转到具体消息
- **Story 3.3**: 搜索范围控制 — 支持全部/主代理/子代理范围
