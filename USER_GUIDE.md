# Helios 操作指南

> 为 macOS 终端打造的 API 调试完全手册。

---

## 目录

1. [安装与启动（macOS）](#1-安装与启动macos)
2. [TUI 交互模式](#2-tui-交互模式)
   - [界面布局](#21-界面布局)
   - [窗口切换](#22-窗口切换)
   - [发送第一个请求](#23-发送第一个请求)
   - [添加 / 编辑参数和请求头](#24-添加--编辑参数和请求头)
   - [设置请求体](#25-设置请求体)
   - [设置认证信息](#26-设置认证信息)
   - [管理集合](#27-管理集合)
   - [导入与导出](#28-导入与导出)
   - [查看历史记录](#29-查看历史记录)
3. [CLI 命令行模式](#3-cli-命令行模式)
4. [快捷键速查表](#4-快捷键速查表)
5. [数据与备份](#5-数据与备份)

---

## 1. 安装与启动（macOS）

Helios 为 macOS 原生优化，支持 Terminal.app、iTerm2、Warp、Kitty 等主流终端。

### 前提条件

确保已安装 Rust（macOS 推荐通过 Homebrew 或 rustup）：

```bash
# 通过 rustup 安装（推荐）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 或通过 Homebrew
brew install rust
```

### 方式一：自动安装脚本（推荐）

```bash
git clone <仓库地址> ~/helios
cd ~/helios
./install.sh
```

脚本会自动编译 release 版本并安装到 `/usr/local/bin/helios`。

### 方式二：Make 安装

```bash
make install
```

### 方式三：手动构建

```bash
cargo build --release
sudo cp target/release/helios /usr/local/bin/
```

### 启动

```bash
# 启动交互式 TUI
helios

# 命令行直接发送请求
helios send GET https://api.example.com/users

# 查看帮助
helios --help
```

### 退出 TUI

- `Ctrl + Q` 或 `Ctrl + C`

---

## 2. TUI 交互模式

### 2.1 界面布局

```
┌────────────────────────────────────────────────────────────────┐
│ ⚡ Helios  v1.0.0                                              │  ← 标题栏
├──────────────┬─────────────────────────────────────────────────┤
│              │  ╭─────╮ ┌─────────────────────────┐ ╭──────╮  │
│  Navigator   │  │ GET │ │ https://httpbin.org/get │ │ SEND │  │  ← URL 栏
│  ─────────   │  ╰─────╯ └─────────────────────────┘ ╰──────╯  │
│  [1]Coll     ├─────────────────────────────────────────────────┤
│  [2]Hist     │  [Params] [Headers] [Body] [Auth]               │  ← Payload
│  [3]Env      │  ─────────────────────────────────────────────  │
│              │  Key              Value               En        │
│  ▶ My API    │  Accept           application/json      ✓       │
│              │                                                 │
│              ├─────────────────────────────────────────────────┤
│              │  Status 200 OK │ 234ms │ 456 bytes             │  ← Response
│              │  {                                              │
│              │    "args": {},                                  │
│              │    "headers": { ... }                           │
│              │  }                                              │
├──────────────┴─────────────────────────────────────────────────┤
│ GET │ Collections │ 0 params │ 1 headers │ Env: OFF           │  ← 状态栏
└────────────────────────────────────────────────────────────────┘
```

四大面板：

| 面板 | 位置 | 作用 |
|------|------|------|
| **Navigator** | 左侧 | 切换 Collections / History / Environments |
| **URL Bar** | 上中 | 设置 HTTP 方法和请求地址 |
| **Payload** | 中右 | 编辑 Params、Headers、Body、Auth |
| **Response** | 下右 | 查看响应状态、耗时、JSON 高亮结果 |

---

### 2.2 窗口切换

| 快捷键 | 作用 |
|--------|------|
| `Tab` | 焦点切换到**下一个**面板（Sidebar → URL Bar → Payload → Response） |
| `Shift + Tab` | 焦点切换到**上一个**面板 |
| `1` / `2` / `3` | 直接切换左侧边栏标签页：Collections / History / Environments |
| `?` | 在状态栏显示完整快捷键帮助 |

**当前焦点高亮**：获得焦点的面板边框会变成**霓虹紫色**，未获得焦点的为暗灰色。

---

### 2.3 发送第一个请求

1. 按 `Tab` 把焦点移到 **URL Bar**
2. 按 `u` 进入 URL 编辑模式，输入接口地址（如 `https://httpbin.org/get`），按 `Enter` 确认
3. （可选）按 `m` 切换 HTTP 方法（GET → POST → PUT → DELETE → PATCH → HEAD → OPTIONS）
4. 按 `Enter` 发送请求
5. 下方 **Response** 面板会显示状态码、耗时、格式化且带语法高亮的 JSON 结果

> 💡 默认已带一个 `Accept: application/json` 的请求头。

---

### 2.4 添加 / 编辑参数和请求头

1. 按 `Tab` 把焦点移到 **Payload** 面板
2. 按 `h` 切换到 **Headers** 标签（或 `p` 切换到 **Params**）
3. **新增一行**：按 `n`，会新增一个空的键值行
4. **编辑键名**：按 `↑`/`↓` 选中一行，按 `e` 输入键名，按 `Enter` 确认
5. **编辑值**：选中一行，按 `v` 输入值，按 `Enter` 确认
6. **启用 / 禁用**：选中一行，按 `Space` 切换，`✓` 表示启用，空白表示禁用
7. **删除一行**：选中一行，按 `d`

> 参数和请求头的操作逻辑完全一致，只是 `p` 和 `h` 两个标签页的区别。

---

### 2.5 设置请求体

1. 在 **Payload** 面板按 `b` 切换到 **Body** 标签
2. 按 `t` 切换 Body 类型：`none` → `json` → `form` → `text` → `xml` → `none`
3. 按 `e` 进入编辑模式，输入 JSON 或其他内容，按 `Enter` 确认

常见组合：

| 场景 | 操作 |
|------|------|
| POST JSON | 方法切到 `POST`，Body 类型切到 `json`，输入 JSON |
| POST 表单 | Body 类型切到 `form`，输入 `key1=value1&key2=value2` |
| 纯文本 | Body 类型切到 `text` |

---

### 2.6 设置认证信息

1. 在 **Payload** 面板按 `a` 切换到 **Auth** 标签
2. **Bearer Token**：按 `e`，输入 Token，按 `Enter` 确认
3. **Basic Auth**：首次按 `e` 输入 Token 后，系统会自动设为 Bearer；如需 Basic，目前需通过 Headers 手动添加 `Authorization: Basic base64(user:pass)`

> Auth 编辑支持实时修改，保存请求时会一并保存认证配置。

---

### 2.7 管理集合

集合（Collection）是多个接口的容器，方便批量管理和导出。

#### 新建集合

- 按 `Ctrl + N`，弹出对话框
- 输入集合名称（如 `Project API`），按 `Enter`

#### 把当前请求保存到集合

1. 配置好请求（URL、方法、Headers、Body）
2. 按 `Ctrl + S`，弹出对话框
3. 输入集合名称（支持名称或 ID），按 `Enter`

#### 在集合下查看请求

1. 按 `1` 切换到 **Collections** 标签
2. 按 `↑`/`↓` 选中一个集合
3. 按 `→` 展开集合，列出该集合下的所有请求
4. 按 `←` 收起集合

---

### 2.8 导入与导出

#### 导出集合

1. 在 **Sidebar** 面板选中要导出的集合（`↑`/`↓`）
2. 按 `Ctrl + E`，弹出对话框
3. 输入格式：`json`（Helios 原生格式）或 `postman`（Postman Collection v2.1）
4. 按 `Enter`，文件会自动保存到当前目录，文件名格式为 `{集合名}.json`

#### 导入集合（CLI 方式）

```bash
# 自动识别格式
helios import ./my_collection.json

# 指定 Postman 格式
helios import ./my_collection.postman_collection.json --format postman
```

导入成功后，重新进入 TUI 即可在 Collections 中看到。

---

### 2.9 查看历史记录

1. 按 `2` 切换到 **History** 标签
2. 按 `↑`/`↓` 浏览历史请求
3. 按 `Enter` 加载某个历史请求到当前编辑区

历史自动保存最近 100 条，包含完整的请求和响应信息。

---

## 3. CLI 命令行模式

适合脚本化、自动化场景，不需要进入 TUI。

### 发送请求

```bash
helios send GET https://httpbin.org/get

helios send POST https://httpbin.org/post \
  -H "Content-Type:application/json" \
  -b '{"name":"test"}'
```

### 集合管理

```bash
helios collection add "My API"
helios collection list
helios collection addreq "My API" GET https://api.example.com/users
helios collection remove "My API"
```

### 环境管理

```bash
helios env add "dev"
helios env set "dev" base_url "https://api.dev.example.com"
helios env list
helios env remove "dev"
```

### 批量运行集合

```bash
helios run "My API"
```

输出示例：
```
Running collection: My API (3 requests)
  GET https://api.example.com/users ... OK (200)
  POST https://api.example.com/users ... OK (201)
  DELETE https://api.example.com/users/1 ... OK (204)

Results: 3 passed, 0 failed
```

### 导入导出

```bash
# 导出为原生 JSON
helios export "My API" --format json --output my_api.json

# 导出为 Postman Collection
helios export "My API" --format postman --output my_api.postman_collection.json

# 导入
helios import ./my_api.json
helios import ./backup.postman_collection.json --format postman
```

---

## 4. 快捷键速查表

### 全局

| 快捷键 | 作用 |
|--------|------|
| `Tab` | 下一面板 |
| `Shift + Tab` | 上一面板 |
| `Ctrl + 1` / `2` / `3` / `4` | 跳转到对应编号面板（Navigator / Request / Payload / Response） |
| `1` / `2` / `3` | Collections / History / Environments |
| `Ctrl + S` | 保存请求到集合（弹窗） |
| `Ctrl + E` | 导出集合（弹窗） |
| `Ctrl + N` | 新建集合（弹窗） |
| `?` | 显示帮助 |
| `Ctrl + Q` / `Ctrl + C` | 退出 |

### URL Bar（上中）

| 快捷键 | 作用 |
|--------|------|
| `m` | 切换 HTTP 方法 |
| `u` | 编辑 URL |
| `Enter` | 发送请求 |

### Payload（中右）

| 快捷键 | 作用 |
|--------|------|
| `p` / `h` / `b` / `a` | Params / Headers / Body / Auth |
| `n` | 新增一行 |
| `d` | 删除选中行 |
| `e` | 编辑键名 / Token |
| `v` | 编辑值 |
| `Space` | 启用 / 禁用 |
| `t` | 切换 Body 类型 |
| `↑` / `↓` | 选择行 |

### Response `[4]`（下右）

| 快捷键 | 作用 |
|--------|------|
| `↑` / `↓` | 滚动内容 |
| `←` / `→` | **切换响应视图**（Body ↔ Headers） |
| `PgUp` / `PgDn` | 快速滚动 |

### 弹窗 / 对话框

| 快捷键 | 作用 |
|--------|------|
| `Enter` | 确认输入 |
| `Esc` | 取消关闭 |

---

## 5. 数据与备份

Helios 严格遵循 macOS 应用数据规范，所有数据存储在：

```
~/Library/Application Support/com.helios.helios/data.json
```

### 备份

```bash
# 备份
cp ~/Library/Application\ Support/com.helios.helios/data.json \
   ~/Desktop/helios_backup.json

# 恢复
cp ~/Desktop/helios_backup.json \
   ~/Library/Application\ Support/com.helios.helios/data.json
```

### 彻底卸载

```bash
make uninstall
rm -rf ~/Library/Application\ Support/com.helios.helios
```
