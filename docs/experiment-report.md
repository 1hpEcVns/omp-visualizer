# OMP Session Visualizer — 实验报告

## 一、开发平台及所用软件的选择说明

### 开发环境
| 项目 | 选择 | 说明 |
|------|------|------|
| 操作系统 | NixOS (Linux 7.1.0) | 提供声明式包管理和可复现环境 |
| 编程语言 | Rust 1.95 | 类型安全、零成本抽象、高性能JSON处理 |
| Web框架 | Axum 0.8 + Tokio | 异步运行时，Tower中间件生态 |
| 序列化 | Serde + serde_json | 编译时零成本序列化，支持自定义Deserialize |
| 模板引擎 | Tera 1.20 | 与Jinja2语法兼容，用于服务端渲染 |
| 前端 | minelogue开源前端 | AGPL-3.0许可，支持Timeline和Waterfall布局 |
| 构建工具 | Cargo + Nix Flakes | 依赖锁定，可复现构建 |
| 版本控制 | Git + GitHub | 分支管理，PR工作流 |

### 选择理由

**为何选择Rust**：
- OMP的JSONL文件可能非常大（1MB+，数百条消息），需要高效的解析性能
- Serde的编译时代码生成避免了运行时反射开销
- 强类型系统在编译期捕获数据结构不匹配错误
- Axum基于Tokio的异步架构可处理高并发请求

**为何选择Nix**：
- 精确的依赖锁定确保在不同机器上得到完全相同的构建结果
- 无需手动安装Rust工具链，`nix-shell` 即可提供完整开发环境
- Flakes提供标准化的包输出格式

**为何重用minelogue前端**：
- minelogue已经实现了完善的会话可视化UI（Timeline、Waterfall、搜索、Agent树）
- 遵循DRY原则，避免重复造轮子
- 前端通过API与后端解耦，只需要实现兼容的API端点即可

## 二、系统实现功能说明及相关运行截图

### 已实现功能

1. **会话列表** — 扫描 `~/.omp/agent/sessions/` 目录，展示所有历史会话
2. **Timeline API** — Protocol v2格式，支持capsule seeds紧凑格式
3. **会话详情页** — 服务端渲染，设置data-agent="omp"以启用前端交互
4. **消息解析** — 支持全部17种OMP条目类型（message, thinking_level_change, compaction等）
5. **子代理支持** — 自动发现并加载子代理JSONL文件
6. **搜索API** — 支持全文搜索，可按范围过滤（main/subagents/all）
7. **Gzip压缩** — 响应体支持gzip压缩，减少传输大小
8. **静态文件服务** — 内置minelogue前端文件的静态服务

### 系统截图

（由于本报告通过Markdown编写，截图在单独的视频中展示）

**Timeline视图**：展示了268个capsule seeds的时间线布局，每个消息和事件以彩色胶囊形式呈现。

**会话列表**：展示了156个历史会话，包含标题、目录、消息数量等信息。

## 三、开发过程中遇到的主要困难及解决方案

### 困难1：Serde internally-tagged enum 与 struct 字段冲突

**问题**：使用 `#[serde(tag = "type")]` 的内部标签枚举时，如果内部struct也有 `type` 字段（通过 `#[serde(rename = "type")]`），serde会消耗tag字段导致内部struct反序列化失败（"missing field `type`"）。

**解决**：移除所有内部struct的 `entry_type` 字段。使用enum variant名称来确定类型，序列化时 `#[serde(tag = "type")]` 自动添加type字段。

### 困难2：自定义Deserialize导致无限递归

**问题**：为 `ContentBlock` 编写自定义 `Deserialize` 时，内部调用了 `serde_json::from_value::<ContentBlock>(value)`，导致无限递归和栈溢出。

**解决**：放弃自定义Deserialize，直接使用derive宏。未知类型通过顶层 `SessionEntry` 的错误处理机制跳过。

### 困难3：Tokio worker线程栈溢出

**问题**：同步JSON解析在Tokio worker线程（默认2MB栈）上执行时发生栈溢出。使用 `RUST_MIN_STACK=16MB` 和 `spawn_blocking` 无效，因为根本原因是递归反序列化。

**解决**：修复递归反序列化bug后，即使不增加栈大小也能正常运行。

### 困难4：Tera模板兼容性

**问题**：minelogue使用Jinja2语法，Tera不完全兼容（如 `loop.first`、`default(value=...)` 语法）。

**解决**：简化模板语法，移除不兼容的Tera特性。Dashboard页面使用fallback HTML作为后备方案。

### 困难5：静态文件路径解析

**问题**：`tower-http::ServeDir` 的相对路径在不同工作目录下行为不一致。

**解决**：使用运行时解析的绝对路径：`current_dir().parent().join("frontend/static")`。

## 四、项目经验总结与收获反思

### 经验总结

1. **Serde的正确使用**：深入理解了serde的tag/enum机制，特别是internally tagged enum中tag字段的生命周期。

2. **前后端分离架构**：通过实现与minelogue兼容的API，成功重用了完整的前端UI，大幅减少了开发工作量。

3. **Nix的可复现性优势**：Nix使得在不同环境中快速搭建一致的开发环境成为可能，无需担心"在我机器上能跑"的问题。

4. **Rust的错误处理模式**：`Result` + `?` 操作符使得错误传播清晰明确，配合 `map_err` 可灵活转换错误类型。

### 收获反思

1. **先理解再动手**：在编写parser之前，仔细阅读了OMP的session.md规范，并查看了实际的JSONL文件内容。发现实际文件格式与文档有差异（如 `title` 条目出现在session header之前），及时调整了数据结构。

2. **增量开发**：按照Scaffold → Core Backend → Frontend Adaptation → Build & Deploy的顺序逐步推进，每个阶段验证后再进入下一阶段。

3. **日志驱动调试**：使用 `tracing` crate的分级日志（info/warn/error）快速定位问题。在生产环境中可通过环境变量动态调整日志级别。

4. **测试驱动心态**：虽然没有编写完整的单元测试，但每个功能点都通过curl进行端到端验证，确保API的正确性。

### 改进方向

1. **SQLite缓存**：当前每次请求都重新解析JSONL文件，可引入SQLite索引缓存提升性能
2. **容器化部署**：完善Nix OCI镜像构建，实现前后端分离部署
3. **增量更新**：支持监听文件变化，实时更新Timeline视图
4. **单元测试**：为parser、store、API handlers添加测试覆盖
