# 我花两周用 Rust 写了一个终端版 Postman —— Helios

> 不是 Postman 用不起，而是终端里敲键盘更有性价比。

---

## 先上效果图

```
┌─[1] Navigator────┬─[2] Request ─────────────────────┐
│ Collections      │ GET  https://httpbin.org/get     │
│ ▼ My API         │                                  │
│   ├─ GET users   ├─[3] Payload ─────────────────────┤
│   ├─ POST login  │ [Params] [Headers] [Body] [Auth] │
│                  │ Key        Value         En      │
│ Environments     │ Accept     application/json  ✓   │
│                  │                                  │
│                  ├─[4] Response ────────────────────┤
│                  │ [ Body ] [ Headers ]             │
│                  │ {                                │
│                  │   "args": {},                    │
│                  │   ...                            │
│                  │ }                                │
└──────────────────┴──────────────────────────────────┘
```

没有鼠标，全程键盘。深海蓝背景 + 霓虹紫强调色，完美融入 macOS Dark Mode。

---

## 为什么要做这个项目？

作为后端开发，我每天要发几十个 HTTP 请求。Postman 很好，但有几个痛点实在忍不了：

1. **太重了** —— 一个 API 客户端动辄几百 MB，启动要等好几秒
2. **鼠标依赖** —— 参数、Header、Body 切来切去，手在键盘和鼠标之间反复横跳
3. **不够酷** —— 作为一个天天泡在 Terminal 里的人，我想在终端里完成一切

所以我决定：用 Rust 写一个**终端原生的 API 客户端**。

两周后，Helios 诞生了。

---

## Helios 是什么？

一句话：**Postman 的终端替代品**，但比 Postman 更快、更轻、更键盘友好。

- **零依赖二进制** —— Rust 编译，单个可执行文件，秒启动
- **纯键盘操作** —— 每个动作都有快捷键，手不离开键盘
- **霓虹暗色主题** —— 深海军蓝底 + 紫色强调，长时间看不累眼
- **数据本地存储** —— `~/Library/Application Support/com.helios.helios/data.json`，Time Machine 自动备份
- **Postman 兼容** —— 支持导入/导出 Postman Collection v2.1

---

## 核心功能

### 1. 四面板 TUI 布局

| 面板 | 快捷键 | 功能 |
|------|--------|------|
| `[1] Navigator` | `x+1` | 侧边栏：Collections / Environments |
| `[2] Request` | `x+2` | URL 栏：Method + 名称 + URL |
| `[3] Payload` | `x+3` | 参数/Header/Body/Auth |
| `[4] Response` | `x+4` | 响应体/响应头，支持滚动和复制 |

`x` 是前缀键，按 `x` 后 2 秒内按 `1-4` 即可切换面板。

### 2. 集合管理

- `f` —— 新建 Collection（像新建文件夹）
- `i` —— 新建请求
- `d` —— 删除（带确认弹窗）
- `c` / `e` —— 切换 Collections / Environments Tab

请求支持命名（按 `n` 编辑名称），保存时自动追踪来源，有来源则覆盖更新，无来源则新增。

### 3. 参数编辑

Params 和 Headers 用表格编辑：
- `i` —— 添加行
- `d` —— 删除行
- `Space` —— 启用/禁用
- `e` / `v` —— 编辑 Key / Value

Headers 还支持**预设循环**：按 `e` 自动循环 `Content-Type`、`Accept`、`Authorization` 等常用 Key，按 `v` 匹配对应 Value，告别手打错字。

### 4. 环境变量

支持多环境管理，变量用 `{{var}}` 语法在 URL/Body 中引用。切换环境后所有请求自动生效。

### 5. 导入导出

```bash
# 导出为 Postman 格式分享给同事
helios export "My API" --format postman --output api.json

# 导入同事的 Postman Collection
helios import ./api.postman_collection.json --format postman
```

---

## 安装（macOS）

```bash
git clone https://github.com/niorw/helios.git && cd helios && ./install.sh
```

或者一行命令：

```bash
make install
```

依赖只有 Rust 工具链，编译后单个二进制文件，随处可运行。

---

## 技术栈

| 模块 | 选型 | 理由 |
|------|------|------|
| 语言 | Rust | 零成本抽象，编译后性能媲美 C |
| TUI | ratatui | Rust 生态最成熟的终端 UI 框架 |
| 终端控制 | crossterm | 跨平台，macOS 支持完美 |
| HTTP | reqwest | 基于 hyper，异步高效 |
| 异步运行时 | tokio | Rust 异步标准 |
| CLI | clap | 声明式参数解析，自动生成帮助 |

---

## 项目结构

```
helios/
├── src/
│   ├── main.rs          # 入口：CLI / TUI 分发
│   ├── cli.rs           # CLI 参数定义
│   ├── models.rs        # 核心数据结构
│   ├── storage.rs       # JSON 持久化
│   ├── http_client.rs   # HTTP 请求执行
│   ├── export_import.rs # Postman / Helios 格式互转
│   ├── utils.rs         # 工具函数
│   └── tui/
│       ├── app.rs       # 业务逻辑
│       ├── events.rs    # 事件路由
│       ├── shortcuts.rs # 快捷键集中管理
│       └── ui.rs        # 纯渲染层
```

设计上严格分层：`app.rs` 管数据、`events.rs` 管按键路由、`shortcuts.rs` 管快捷键定义、`ui.rs` 只管画，互不越界。

---

## 为什么用 Rust？

很多人问我：一个 API 客户端，用 Python/Node 写不是更快吗？

是更快，但体验完全不同：

- **启动速度**：Rust 二进制冷启动 < 100ms，Electron 应用动辄 3-5 秒
- **内存占用**：Helios 运行时内存 < 20MB，Postman 轻松 300MB+
- **单文件分发**：编译后一个 `helios` 可执行文件，scp 到任何机器直接跑
- **类型安全**：编辑器里的请求模型、状态机全部编译期检查，运行时不会崩

说到底，**工具类软件就应该像工具一样——拿起来就能用，用完不拖泥带水。**

---

## 后续计划

- [ ] 环境变量替换在 Body 中的实时预览
- [ ] 请求历史搜索
- [ ] 响应 Body JSON 折叠/展开
- [ ] 插件系统（Lua/JS 脚本）
- [ ] Linux / Windows 支持

---

## 开源地址

**GitHub**: https://github.com/niorw/helios

欢迎 Star、提 Issue、扔 PR。如果你也是终端重度用户，这个项目就是为你写的。

---

*用终端的人，都有一颗追求效率的心。*
