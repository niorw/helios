# Helios V3 PRD — 产品需求文档

> 版本: 3.0 | 日期: 2026-05-16 | 作者: Eddie
> 竞品参考: Bruno v3.3.0 (43.8k star) / HTTPie (38.1k star) / Hurl (18.9k star)

---

## 一、项目背景与定位

### 1.1 问题定义

当前 API 客户端市场存在两个极端：

| 极端 | 代表 | 问题 |
|------|------|------|
| 臃肿平台化 | Postman | 强制登录、云端同步隐私风险、功能堆砌、免费版限制多 |
| 极简命令行 | HTTPie / curl | 只能发请求，无法管理集合、环境、测试 |

Bruno 证明了一个中间路线的巨大市场：本地优先 + Git 协作 + 纯文本集合。但 Bruno 是 GUI（Electron），仍然需要离开终端。

### 1.2 Helios 定位

Helios 是唯一一个 TUI（终端用户界面）形态的 Git-Native API 客户端。

目标用户画像：
- 日常在终端工作的后端开发者（vim/helix/lazygit 用户）
- 需要在 SSH 远程服务器上调试 API 的 DevOps 工程师
- 追求零鼠标、键盘驱动工作流的极客开发者
- 对 Postman 云同步有安全顾虑的企业开发者

### 1.3 核心差异化

| 维度 | Bruno | Helios |
|------|-------|--------|
| 交互形态 | Electron GUI | Ratatui TUI |
| 运行环境 | 桌面系统 | 任何终端（含 SSH） |
| 资源占用 | ~300MB RAM | ~10MB RAM |
| 启动速度 | ~2s | ~50ms |
| SSH 可用 | 否 | 是 |
| 脚本引擎 | JavaScript (V8) | 声明式 DSL |
| 集合格式 | .bru 自定义 DSL | .helios.yaml 标准 YAML |
| Git 集成 | GUI 内嵌 Git 操作 | CLI 原生 + TUI 变更感知 |

---

## 二、竞品深度分析：Bruno 功能全景

### 2.1 Bruno 完整功能矩阵

| 功能域 | 子功能 | Bruno 现状 | Helios 现状 | 差距 |
|--------|--------|-----------|-------------|------|
| 请求协议 | REST | 完整 | 完整 | 无 |
| | GraphQL + Query Builder | 可视化 Builder + 变量 | 基础 GraphQL Body | 大 |
| | gRPC (Unary/4种流) | Proto文件 + Server Reflection | 无 | 极大 |
| | WebSocket | 连接管理 + 消息历史 | 无 | 极大 |
| | SOAP / WSDL | WSDL导入 + XML Envelope | 无 | 大 |
| 请求构建 | URL + Method + Params | 完整 | 完整 | 无 |
| | Body (JSON/XML/Form/Multipart/Raw/Binary) | 7种类型 | 6种(Json/Form/Text/Xml/Graphql/FormData/None) | 小 |
| | Headers (批量编辑/预设) | 完整 | 预设循环 | 小 |
| | Auth (7种协议) | Basic/Bearer/Digest/NTLM/OAuth1/OAuth2/AWSSigV4 | Basic/Bearer | 极大 |
| | Cookie 管理 | 完整(Domain/Path/Secure/HTTPOnly) | 无 | 大 |
| | 代码生成 (35+语言) | 内置 | curl 导出 | 大 |
| | 请求设置 (超时/重定向/URL编码) | 完整 | 无 | 中 |
| 变量系统 | 全局/环境/集合/文件夹/请求/运行时/提示 7层 | 完整7层+优先级 | 环境变量+内置动态变量 | 极大 |
| | 变量插值 (对象/数组/日期) | 对象点号+数组索引+Date | 字符串插值 | 大 |
| | 进程环境变量 | process.env.VAR | 无 | 中 |
| | 提示变量 {{?Prompt}} | 运行时弹窗输入 | 无 | 中 |
| | 集合变量 | 有 | 无 | 中 |
| | 文件夹变量 | 有 | 无 | 中 |
| 脚本 | JavaScript Pre/Post Script | V8引擎完整JS | 声明式 @header/@set | 大 |
| | 脚本执行流 (Sandwich/Sequential) | 2种模式可选 | 无 | 中 |
| | 请求链 (bru.runRequest/bru.sendRequest) | 有 | 依赖链(简单) | 中 |
| | bru.setNextRequest 条件跳转 | 有 | 无 | 中 |
| 测试 | Assertions (基于Chai) | 完整Chai断言 | 6种断言操作符 | 中 |
| | Collection Runner | GUI内运行+无限次 | CLI run | 小 |
| | 数据驱动测试 (CSV/JSON) | CSV + JSON | 无 | 大 |
| | 标签过滤运行 (--tags/--exclude-tags) | 有 | 无 | 中 |
| | 并行执行 (--parallel) | 有 | 无 | 中 |
| Git协作 | 文件系统集合 | 核心功能 | data.json 单文件 | 极大 |
| | GUI Git操作 | Pull/Push/Branch/Stash/Diff | 无 | 大 |
| | Git Provider 集成 (GitHub/GitLab/Bitbucket) | 有 | 无 | 中 |
| | Fetch in Bruno 按钮 | 可嵌入网页 | 无 | 小 |
| 安全 | Secret Variables | 环境变量标记secret | 无 | 极大 |
| | .env 文件 | 支持 | 无 | 中 |
| | Secret Manager (AWS/Azure/HashiCorp Vault) | 3种云密钥管理 | 无 | 大 |
| | Secret 脱敏 (报告中遮蔽) | 有 | 无 | 中 |
| | 客户端证书 (mTLS) | PEM/PKCS12 | 无 | 中 |
| 文档 | 集合/文件夹/请求4级文档 | Markdown编辑器+预览 | 无 | 大 |
| | 自动生成文档 | HTML文档生成 | 无 | 大 |
| OpenAPI | 导入/导出/设计OAS | 完整3.0+2.0 | 导入3.0 | 中 |
| | OpenAPI Sync (远程规范同步) | 有(免费5次/月) | 无 | 中 |
| 开发者工具 | Dev Tools (Console/Network/Performance/Terminal) | 4标签 | 无 | 大 |
| | Timeline (请求时序) | Request/Response/Network 3标签 | 无 | 大 |
| | 响应示例保存 | 保存多次响应为Example | 无 | 中 |
| CLI | bru run 集合运行 | 完整 | helios run | 小 |
| | CSV/JSON数据驱动 | 有 | 无 | 大 |
| | 报告 (JSON/JUnit/HTML) | 3格式+脱敏 | JSON报告 | 中 |
| | CI集成 (GitHub Actions/Jenkins) | 脚手架生成 | 无 | 大 |
| 集合格式 | .bru DSL | 自定义标记语言 | data.json | 大 |
| | OpenCollection YAML | 行业标准YAML(新) | 无 | 大 |
| 转换器 | Postman/Insomnia/OpenAPI/WSDL → Bruno | 4种 | Postman导出+OpenAPI导入 | 中 |
| 扩展 | VS Code Extension | 完整 | 无 | 大 |
| | AI Agent集成 (Cursor/Copilot/Codex/Claude) | 4种 | 无 | 大 |
| | 插件系统 | 无 | 无 | 持平 |

### 2.2 Bruno 界面布局分析

```
+---------------------------------------------------------------+
| 菜单栏: File | Edit | Selection | View | Go | Run | Debug    |
+----------+----------------------------------------------------+
|          |  Tab Bar: [GET Users] [POST Login] [+]            |
| 集合     +----------------------------------------------------+
| 文件树   |  [GET v] [https://api.example.com/users] [Send]   |
|          |  +---Params---+---Headers---+---Body---+---Auth--+|
| > 🌐 API |  | Key  | Value | Desc  |[x]                  ||
|   ├ GET  |  | page | 1     |       |[x]                  ||
|   │ users|  | size | 20    |       |[x]                  ||
|   ├ POST |  +------+-------+-------+---------------------+|
|   │ login|  | Script | Tests | Vars | Docs | Settings     ||
|          |  | // Pre-request script                        ||
| 环境选择 |  | bru.setVar('ts', Date.now());                ||
| [Dev  v] |  +----------------------------------------------+|
+----------+  Response                                       |
|          |  Status: 200 OK | Time: 156ms | Size: 1.2KB     |
|          |  +---Body---+---Headers---+---Timeline---+       |
|          |  | {                                           ||
|          |  |   "users": [...],                           ||
|          |  |   "total": 42                               ||
|          |  | }                                           ||
|          |  +---------------------------------------------+|
+----------+----------------------------------------------------+
| Console | Network | Performance | Terminal                   |
+---------------------------------------------------------------+
```

Bruno 的关键界面设计特点：

1. 左侧栏：集合文件树（带HTTP方法彩色标记）+ 环境选择器
2. 主区域上方：Tab标签页 + URL工具栏（方法+URL+Send）
3. 主区域中部：请求配置标签页（Params/Headers/Body/Auth/Script/Tests/Vars/Docs/Settings）
4. 主区域下方：响应面板（Body/Headers/Timeline）+ DevTools
5. DevTools底部面板：Console/Network/Performance/Terminal

### 2.3 Helios 当前界面布局

```
+---------------------------------------------------------------+
| Title Bar: Helios | Collection:xxx | Env:xxx                 |
+----------+----------------------------------------------------+
|          |  URL Bar: [GET v] [url____________] [Enter=Send]  |
| Sidebar  +----------------------------------------------------+
| [1]Coll  |  Payload Tabs: [Params] [Headers] [Body] [Auth]  |
| [2]Env   |  +------+-------+-------+                        |
|          |  | Key  | Value | [x]   |                        |
| > 🌐 API |  |      |       |       |                        |
|   ├ GET  |  +------+-------+-------+                        |
|   │ users|                                                    |
|   ├ POST +----------------------------------------------------+
|   │ login|  Response Tabs: [Body] [Headers]                  |
|          |  Status: 200 | 156ms                               |
|          |  { "users": [...] }                                 |
+----------+----------------------------------------------------+
| Shortcut Bar: m1-4=Pane Tab=Cycle /=Search Ctrl+S=Save      |
+---------------------------------------------------------------+
```

Helios 缺失的界面元素：
- 请求级标签页：Script / Tests / Vars / Docs / Settings
- 响应级标签页：Timeline / Cookies / Examples
- DevTools 底部面板：Console / Network / Performance
- 集合文件树中的文件夹层级
- 环境选择器的快速切换

---

## 三、产品架构升级

### 3.1 存储架构重构（从 data.json 到文件系统）

当前 Helios 所有数据存储在单一 data.json 文件中，这是最大的架构瓶颈。
必须重构为文件系统存储，与 Bruno 的 .bru / OpenCollection YAML 对齐。

#### 目标存储结构

```
my-api-project/                    # 集合根目录 (= 一个 Bruno Collection)
├── bruno.json                     # 集合元信息 (name, version, scripts.flow)
├── .helios/                       # Helios 元数据目录 (gitignored 敏感数据)
│   ├── secrets.enc                # 加密的密钥存储
│   ├── snapshots/                 # 请求快照
│   └── cache/                     # 缓存 (schema, introspection)
├── environments/                  # 环境配置
│   ├── development.yml            # 开发环境
│   ├── staging.yml                # 预发布环境
│   └── production.yml             # 生产环境 (secret 标记的值不写入)
├── auth/                          # 认证配置
│   └── oauth2.yml                 # OAuth2 凭据配置
├── users/                         # 文件夹 (= Bruno Folder)
│   ├── list-users.helios.yml      # 请求文件
│   ├── create-user.helios.yml
│   └── get-user.helios.yml
├── auth/                          # 另一个文件夹
│   ├── login.helios.yml
│   └── refresh-token.helios.yml
├── collection.yml                 # 集合级变量和脚本
└── .heliosignore                  # 排除规则
```

#### .helios.yml 请求文件格式

```yaml
# 请求文件格式 - 标准 YAML，对齐 Bruno OpenCollection YAML
info:
  name: 创建用户
  type: http
  seq: 1
  tags: [smoke, auth]

http:
  method: POST
  url: "{{base_url}}/users"
  params:
    - key: expand
      value: profile
      enabled: true
  headers:
    - key: Content-Type
      value: application/json
      enabled: true
    - key: Authorization
      value: "Bearer {{auth_token}}"
      enabled: true
  body:
    type: json
    content: |
      {
        "name": "{{?用户名}}",
        "email": "{{$randomEmail}}",
        "role": "user"
      }
  auth:
    type: bearer
    token: "{{vault:api_token}}"

runtime:
  pre_request:
    - action: set_header
      key: X-Request-ID
      value: "{{$uuid}}"
    - action: set_var
      key: timestamp
      value: "{{$timestamp}}"
  post_response:
    - action: extract
      var_name: user_id
      json_path: "$.id"
    - action: extract
      var_name: auth_token
      json_path: "$.token"
  tests:
    - name: 状态码为201
      assert: status
      operator: equals
      expected: "201"
    - name: 返回用户ID
      assert: body.id
      operator: exists
    - name: 响应时间小于500ms
      assert: response_time
      operator: less_than
      expected: "500"

settings:
  timeout: 30000
  follow_redirects: true
  max_redirects: 5
  encode_url: true

docs: |
  ## 创建用户接口

  用于注册新用户，需要管理员权限。

  ### 请求示例
  ```bash
  curl -X POST {{base_url}}/users \
    -H "Authorization: Bearer {{auth_token}}" \
    -d '{"name":"test","email":"test@example.com"}'
  ```
```

#### 集合级配置 collection.yml

```yaml
name: 我的API集合
version: "1.0.0"

variables:
  base_url: "https://api.example.com"
  api_version: "v2"

auth:
  type: bearer
  token: "{{vault:api_token}}"

runtime:
  pre_request:
    - action: set_header
      key: X-API-Version
      value: "{{api_version}}"
  post_response: []

scripts:
  flow: sandwich    # sandwich | sequential

docs: |
  # 我的API集合文档

  这是一组用于管理用户的API接口。
```

#### 环境文件 environments/development.yml

```yaml
name: 开发环境
color: "#4CAF50"      # 环境颜色标识

variables:
  base_url: "https://api.dev.example.com"
  api_version: "v2"
  timeout: "30000"

secrets:
  - api_token         # 密钥名，实际值存储在 .helios/secrets.enc 中
```

### 3.2 TUI 界面重构

#### 新增界面元素

现有 4 个请求标签页扩展为 8 个：

| 标签 | 快捷键 | 功能 | 对标 Bruno |
|------|--------|------|-----------|
| Params | p | 查询参数 | 已有 |
| Headers | h | 请求头 | 已有 |
| Body | b | 请求体(7种类型) | 已有 |
| Auth | a | 认证配置(7种协议) | 增强 |
| Script | s | 前后置脚本 | 新增(对齐Bruno Script tab) |
| Tests | t | 测试断言 | 新增(对齐Bruno Tests tab) |
| Vars | v | 变量管理 | 新增(对齐Bruno Vars tab) |
| Docs | d | 请求文档 | 新增(对齐Bruno Docs tab) |

响应面板扩展为 5 个标签页：

| 标签 | 快捷键 | 功能 | 对标 Bruno |
|------|--------|------|-----------|
| Body | b | 响应体 | 已有 |
| Headers | h | 响应头 | 已有 |
| Timeline | t | 请求时序 | 新增(对齐Bruno Timeline) |
| Cookies | c | Cookie管理 | 新增(对齐Bruno Cookies) |
| Examples | e | 响应示例 | 新增(对齐Bruno Response Examples) |

#### 新增底部 DevTools 面板

按 `Ctrl+D` 切换 DevTools 面板显示/隐藏：

| 标签 | 功能 | 对标 Bruno |
|------|------|-----------|
| Console | 脚本日志输出 | Bruno Console |
| Network | 网络请求详情 | Bruno Network |
| Timeline | DNS/TCP/TLS/TTFB时序 | Bruno Timeline |

#### 新增侧边栏标签页

| 标签 | 快捷键 | 功能 |
|------|--------|------|
| Collections | 1 | 集合文件树(已有) |
| History | 2 | 请求历史(已有) |
| Environments | 3 | 环境管理(已有) |
| Git | 4 | Git状态与操作(新增) |

---

## 四、功能需求详细设计

### Phase 0：基础设施（P0，2周）

---

#### F01: .helios.yml 文件格式与存储层重构

**用户故事**：作为开发者，我希望集合以文件形式存储在我的项目目录中，这样我可以用 Git 管理和版本控制 API 定义。

**功能规格**：

1. 定义 .helios.yml 文件格式（基于 YAML，对齐 Bruno OpenCollection 规范）
2. 一个文件 = 一个请求，一个目录 = 一个集合
3. 集合根目录放置 bruno.json（兼容 Bruno）或 collection.yml（Helios 原生）
4. 环境配置存储在 environments/ 子目录
5. 实现 Storage trait 的文件系统后端（FileStorage）
6. 保持对旧 data.json 格式的读取兼容
7. `helios migrate` 命令将 data.json 转换为文件系统结构
8. `helios init <project-name>` 生成项目骨架

**文件格式规范**：

```yaml
# .helios.yml 请求文件
info:
  name: string           # 请求名称
  type: http|graphql|grpc|websocket  # 请求类型
  seq: number            # 排序序号
  tags: string[]         # 标签列表

http:
  method: GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS
  url: string            # 支持变量插值 {{var}}
  params:                # 查询参数
    - key: string
      value: string
      enabled: bool
  headers:               # 请求头
    - key: string
      value: string
      enabled: bool
  body:                  # 请求体
    type: none|json|xml|text|form-urlencoded|multipart-form|graphql
    content: string      # body 内容
    graphql_query: string   # GraphQL query（type=graphql时）
    graphql_variables: string  # GraphQL variables
    form_data:              # multipart 表单
      - key: string
        value: string
        is_file: bool
        file_path: string
  auth:                  # 认证配置
    type: none|basic|bearer|digest|oauth2|aws-sigv4
    # 根据类型不同有不同字段

runtime:
  pre_request:           # 前置操作列表
    - action: set_header | set_var | set_param
      key: string
      value: string
  post_response:         # 后置操作列表
    - action: extract | set_var | assert
      var_name: string   # extract/set_var 时
      json_path: string  # extract 时
      assert: string     # assert 时: status|body.xxx|header.xxx|response_time
      operator: equals|not_equals|contains|greater_than|less_than|exists
      expected: string   # assert 时
  tests:                 # 测试断言列表
    - name: string
      assert: string
      operator: string
      expected: string

settings:
  timeout: number        # 毫秒，默认30000
  follow_redirects: bool # 默认true
  max_redirects: number  # 默认5
  encode_url: bool       # 默认true

docs: string             # Markdown 文档
```

**验收标准**：
- [ ] `helios init my-api` 生成标准项目骨架
- [ ] `helios migrate` 将 data.json 转换为文件系统结构
- [ ] TUI 正确加载文件系统存储的集合
- [ ] 旧 data.json 格式可读取但标记为 deprecated
- [ ] .helios.yml 文件可通过 yamllint 验证

**测试用例**：
```
test_helios_yml_parse_basic_request
test_helios_yml_parse_with_auth
test_helios_yml_parse_with_tests
test_helios_yml_roundtrip
test_migrate_data_json_to_filesystem
test_init_project_skeleton
test_load_collection_from_directory
test_yaml_syntax_validation
```

**涉及文件**：helios_format.rs(新), storage.rs, cli.rs, models.rs

---

#### F02: 集合即目录 + 文件夹层级

**用户故事**：作为开发者，我希望集合的文件夹结构直接映射文件系统目录，这样我可以像管理代码一样管理 API 请求。

**功能规格**：

1. 集合根目录 = 顶层集合节点
2. 子目录 = 文件夹（可无限嵌套）
3. .helios.yml 文件 = 请求
4. 文件夹内可放置 folder.yml 定义文件夹级变量和脚本
5. 侧边栏文件树展示目录层级
6. 支持拖拽排序（通过修改 seq 字段）
7. `helios collection tree` 输出集合树形结构
8. 新建请求自动创建 .helios.yml 文件
9. 删除请求自动删除对应文件

**folder.yml 格式**：

```yaml
name: 用户管理
seq: 1

variables:
  user_base: "/users"

runtime:
  pre_request: []
  post_response: []

docs: |
  # 用户管理接口组
```

**验收标准**：
- [ ] TUI 侧边栏正确显示嵌套文件夹
- [ ] 文件夹展开/折叠正常
- [ ] 新建请求在正确目录下创建文件
- [ ] 删除请求删除对应文件
- [ ] 文件夹级变量在子请求中可用

**测试用例**：
```
test_directory_to_collection_mapping
test_nested_folder_hierarchy
test_folder_yml_variables_available
test_create_request_creates_file
test_delete_request_deletes_file
test_collection_tree_output
```

**涉及文件**：storage.rs, app.rs, ui.rs, cli.rs

---

#### F03: 密钥保险箱

**用户故事**：作为开发者，我希望 API 密钥等敏感信息不被写入版本控制，但仍然可以在请求中使用。

**功能规格**：

1. 密钥存储在 .helios/secrets.enc（AES-256-GCM 加密）
2. macOS 优先使用 Keychain Services 存储
3. `{{vault:key_name}}` 语法从保险箱取值
4. `helios vault set <key> <value>` 设置密钥
5. `helios vault list` 列出密钥名（不显示值）
6. `helios vault get <key>` 获取密钥值（需确认）
7. `helios vault delete <key>` 删除密钥
8. 环境文件中 secrets 列表只记录密钥名，不记录值
9. 导出集合时密钥不导出
10. 报告中密钥值自动遮蔽为 ***

**密钥优先级**：vault 引用 > 环境变量 > 集合变量 > 全局变量

**加密方案**：
- 主密钥派生自 macOS Keychain 或用户设置的 passphrase
- 每条密钥独立 IV + AES-256-GCM 加密
- 加密文件权限 600

**验收标准**：
- [ ] vault set/get/list/delete 正常工作
- [ ] {{vault:xxx}} 在请求中正确替换
- [ ] 密钥不写入 .helios.yml 文件
- [ ] 导出集合不含密钥
- [ ] macOS Keychain 集成可用
- [ ] 降级方案：passphrase + 文件加密

**测试用例**：
```
test_vault_set_and_get
test_vault_list_shows_names_only
test_vault_delete_removes_key
test_vault_variable_in_request_url
test_vault_variable_in_header
test_vault_not_in_export
test_vault_not_in_yml_file
test_vault_masking_in_report
test_vault_keychain_integration
test_vault_passphrase_fallback
```

**涉及文件**：vault.rs(新), models.rs, storage.rs, http_client.rs, cli.rs

---

### Phase 1：开发者体验核心（P1，3周）

---

#### F04: 智能补全引擎

**用户故事**：作为开发者，我在输入 URL、Header、变量时希望有自动补全，减少打字和记忆负担。

**功能规格**：

1. URL 补全：
   - 输入时匹配历史 URL
   - 输入时匹配当前集合中的 URL
   - 环境变量补全（输入 {{ 触发）
   
2. Header 补全：
   - Key 补全：Content-Type, Accept, Authorization 等 50+ 常用 Header
   - Value 补全：Content-Type → application/json, multipart/form-data 等
   
3. 变量名补全：
   - 输入 {{ 后列出所有可用变量（环境/集合/全局/vault）
   - 显示变量来源标注
   
4. JSON Body 字段补全：
   - 基于上次响应的 JSON Schema 推断字段
   - 缓存 Schema 到 .helios/cache/
   
5. Shell 补全脚本：
   - `helios completion bash > /etc/bash_completion.d/helios`
   - `helios completion zsh > ~/.zfunc/_helios`
   - `helios completion fish > ~/.config/fish/completions/helios.fish`

**TUI 补全交互**：
- 输入时自动弹出补全列表（最多显示 8 项）
- 上下键选择，Tab/Enter 确认
- Esc 取消
- Ctrl+N/Ctrl+P 也可选择

**验收标准**：
- [ ] URL 输入时出现历史和集合 URL 补全
- [ ] {{ 触发变量名补全列表
- [ ] Header Key/Value 有预设补全
- [ ] Shell 补全脚本可生成
- [ ] 补全列表不影响正常输入速度

**测试用例**：
```
test_complete_url_from_history
test_complete_url_from_collection
test_complete_variable_names
test_complete_header_keys
test_complete_header_values
test_complete_json_body_fields
test_shell_completion_bash
test_shell_completion_zsh
test_shell_completion_fish
```

**涉及文件**：completion.rs(新), ui.rs, events.rs, cli.rs

---

#### F05: 请求标签与分类

**用户故事**：作为开发者，我希望给请求打标签来分类和过滤，比如 smoke、auth、regression。

**功能规格**：

1. Request 新增 `tags: Vec<String>` 字段
2. .helios.yml 中 info.tags 数组
3. 侧边栏支持按标签过滤（按 `T` 切换标签过滤模式）
4. `helios run --tags smoke,auth` 只运行匹配标签的请求
5. `helios run --exclude-tags regression` 排除标签
6. `helios tag list` 查看所有标签及关联请求数
7. TUI 中标签以彩色小标签显示在请求名旁
8. 标签颜色自动生成（基于标签名 hash）

**验收标准**：
- [ ] 可给请求添加多个标签
- [ ] 侧边栏标签过滤正常
- [ ] CLI --tags/--exclude-tags 过滤运行
- [ ] 标签在 .helios.yml 中正确存储

**测试用例**：
```
test_tag_add_to_request
test_tag_multiple_tags
test_tag_filter_in_sidebar
test_tag_filter_run_cli
test_tag_exclude_run_cli
test_tag_list_command
test_tag_color_generation
test_tag_in_helios_yml
```

**涉及文件**：models.rs, app.rs, ui.rs, cli.rs

---

#### F06: 性能剖析器（Timeline）

**用户故事**：作为开发者，我希望看到每个请求的详细时序（DNS/TCP/TLS/TTFB/传输），帮我定位性能瓶颈。

**功能规格**：

1. 每次请求记录详细时序：
   - DNS 查询时间
   - TCP 连接时间
   - TLS 握手时间
   - 请求发送时间
   - 等待首字节时间 (TTFB)
   - 内容下载时间
   - 总耗时
2. Response 面板新增 Timeline 标签页
3. Timeline 以瀑布图展示各阶段
4. 状态栏显示总耗时和 TTFB
5. `helios perf <request>` 输出详细性能数据
6. 性能数据持久化到历史记录
7. 支持对比同一请求在不同时间的性能

**数据结构**：

```rust
pub struct RequestTiming {
    pub dns_lookup: Duration,
    pub tcp_connect: Duration,
    pub tls_handshake: Duration,
    pub request_sent: Duration,
    pub time_to_first_byte: Duration,
    pub content_download: Duration,
    pub total: Duration,
}
```

**TUI Timeline 渲染**：

```
Timeline ─────────────────────────────────────
DNS Lookup   ████                          12ms
TCP Connect      █████                     18ms
TLS Handshake       ████████              32ms
Request Sent               █              2ms
Waiting (TTFB)              ████████████  45ms
Download                                ████  8ms
──────────────────────────────────────────────
Total: 117ms
```

**验收标准**：
- [ ] 每次请求记录6个时序阶段
- [ ] TUI Timeline 标签页正常展示
- [ ] 瀑布图按比例渲染
- [ ] 性能数据写入历史记录
- [ ] CLI perf 命令正常输出

**测试用例**：
```
test_timing_record_on_request
test_timeline_render_basic
test_timeline_render_slow_dns
test_perf_cli_output
test_timing_persisted_in_history
test_timing_comparison
```

**涉及文件**：perf.rs(新), http_client.rs, ui.rs, models.rs, cli.rs

---

#### F07: CI 配置文件

**用户故事**：作为开发者，我希望用配置文件定义 API 测试流水线，直接在 CI 中运行。

**功能规格**：

1. 项目根目录放置 .helios-ci.yml
2. 配置文件定义：

```yaml
# .helios-ci.yml
version: "1.0"

collections:
  - path: ./my-api
    environment: staging
    tags: [smoke, regression]
    exclude_tags: [manual]
    
  - path: ./admin-api
    environment: staging

reporting:
  - format: junit
    output: reports/junit.xml
  - format: html
    output: reports/api-test.html
    
thresholds:
  pass_rate: 80          # 80% 通过率
  max_response_time: 5000 # 5秒最大响应时间

notifications:
  on_failure: true
  
env_overrides:
  base_url: "{{CI_API_URL}}"
```

3. `helios ci run` 执行 CI 配置
4. `helios ci check` 预检（语法验证、断言覆盖率）
5. 退出码：0=全部通过, 1=断言失败, 2=请求错误, 3=配置错误
6. `helios ci scaffold github` 生成 GitHub Actions workflow
7. `helios ci scaffold gitlab` 生成 .gitlab-ci.yml
8. 支持 --junit-output / --html-output / --json-output
9. 报告中密钥自动脱敏
10. 支持 --reporter-skip-headers 跳过敏感头

**GitHub Actions 脚手架**：

```yaml
# .github/workflows/api-test.yml (自动生成)
name: API Tests
on: [push, pull_request]
jobs:
  api-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Helios
        run: curl -sSL https://helios.dev/install.sh | sh
      - name: Run API Tests
        run: helios ci run
      - name: Upload Report
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: api-test-report
          path: reports/
```

**验收标准**：
- [ ] .helios-ci.yml 语法正确解析
- [ ] helios ci run 按配置执行
- [ ] 退出码语义正确
- [ ] JUnit XML 输出 CI 兼容
- [ ] HTML 报告可读
- [ ] 密钥在报告中脱敏
- [ ] GitHub Actions 脚手架可用

**测试用例**：
```
test_ci_config_parse_basic
test_ci_config_parse_with_thresholds
test_ci_run_success_exit_0
test_ci_run_assertion_fail_exit_1
test_ci_run_request_error_exit_2
test_ci_config_error_exit_3
test_junit_output_format
test_html_report_generation
test_secret_masking_in_report
test_scaffold_github_actions
test_scaffold_gitlab_ci
```

**涉及文件**：ci_config.rs(新), ci_scaffold.rs(新), report.rs, cli.rs

---

#### F08: 7种认证协议支持

**用户故事**：作为开发者，我希望 Helios 支持我项目所需的各种认证方式，而不只是 Basic 和 Bearer。

**功能规格**：

1. Auth 枚举扩展为 7 种：

| 类型 | 字段 | 说明 |
|------|------|------|
| None | - | 无认证 |
| Basic | username, password | HTTP Basic |
| Bearer | token | Bearer Token |
| Digest | username, password | HTTP Digest |
| OAuth2 | grant_type, auth_url, token_url, client_id, client_secret, scopes, redirect_uri | OAuth 2.0 |
| AwsSigV4 | access_key, secret_key, region, service | AWS Signature V4 |
| ApiKey | key, value, add_to(header/query) | API Key |

2. OAuth 2.0 详细设计（对齐 Bruno）：
   - 支持 4 种 Grant Type：Authorization Code, Client Credentials, Password, Implicit
   - 自动 Token 注入到请求头或参数
   - 自动 Token 刷新（过期前 60 秒）
   - Token 数据访问：{{$oauth2.access_token}}
   - 本地回调服务器（Authorization Code 流程，监听随机端口）
   - 系统浏览器打开授权页面
   - 认证状态持久化

3. AWS SigV4 详细设计：
   - 自动计算签名
   - 支持 {{vault:aws_access_key}} 引用密钥
   - 支持自定义 region 和 service

4. 集合级 Auth：在 collection.yml 中定义，所有请求继承
5. 文件夹级 Auth：在 folder.yml 中定义，子项继承
6. 请求级 Auth 可覆盖上层

**验收标准**：
- [ ] 7 种 Auth 类型在 TUI Auth 标签页可选
- [ ] OAuth2 Client Credentials 流程正常
- [ ] OAuth2 Authorization Code 流程正常（系统浏览器）
- [ ] OAuth2 自动刷新 Token
- [ ] AWS SigV4 签名正确
- [ ] 集合/文件夹/请求三级 Auth 继承
- [ ] Auth 配置在 .helios.yml 中正确存储

**测试用例**：
```
test_auth_basic_request
test_auth_bearer_request
test_auth_digest_request
test_auth_oauth2_client_credentials
test_auth_oauth2_authorization_code
test_auth_oauth2_token_refresh
test_auth_oauth2_auto_inject
test_auth_aws_sigv4_signing
test_auth_api_key_in_header
test_auth_api_key_in_query
test_auth_collection_level_inheritance
test_auth_folder_level_inheritance
test_auth_request_level_override
test_auth_in_helios_yml
```

**涉及文件**：auth.rs(新), models.rs, http_client.rs, ui.rs, cli.rs

---

### Phase 2：协作与安全（P2，3周）

---

#### F09: Git 感知与变更追踪

**用户故事**：作为开发者，我希望在 TUI 中看到 API 请求的 Git 状态，知道哪些请求被修改了。

**功能规格**：

1. `helios git status` 显示请求文件变更（新增/修改/删除）
2. `helios git diff` 显示请求前后差异（复用 diff 模块）
3. TUI 侧边栏文件图标显示 Git 状态：
   - M 黄色 = 已修改
   - A 绿色 = 新增
   - D 红色 = 已删除
   - ? 灰色 = 未追踪
4. 按 `G` 在当前请求查看 Git 状态和 diff
5. `helios git check` 检测潜在敏感信息泄露
6. 集成 .heliosignore 排除规则
7. pre-commit hook：`helios git check --staged`

**验收标准**：
- [ ] 侧边栏显示 Git 状态标记
- [ ] helios git status 输出变更列表
- [ ] helios git diff 输出请求差异
- [ ] helios git check 检测到 API Key 模式

**测试用例**：
```
test_git_status_modified
test_git_status_new
test_git_status_deleted
test_git_diff_request
test_git_check_detects_api_key
test_git_check_detects_password
test_heliosignore_patterns
test_pre_commit_hook
```

**涉及文件**：git_integration.rs(新), app.rs, ui.rs, cli.rs

---

#### F10: 变量系统升级（7层变量 + 提示变量）

**用户故事**：作为开发者，我希望有更精细的变量作用域控制，不同层级覆盖不同场景。

**功能规格**：

1. 7 层变量（优先级从高到低）：
   - 运行时变量（脚本中 bru.setVar 设置）
   - 请求变量（.helios.yml 的 runtime.pre_request.set_var）
   - 文件夹变量（folder.yml 的 variables）
   - 环境变量（environments/xxx.yml）
   - 集合变量（collection.yml 的 variables）
   - 全局变量（~/.helios/global-vars.yml）
   - 提示变量（{{?Prompt Message}}，运行时输入）

2. 变量插值增强：
   - 对象点号访问：{{user.username}}
   - 数组索引访问：{{apiTypes[0]}}
   - Date 对象：自动转 ISO 字符串
   - 布尔/数字保持原生类型

3. 进程环境变量：{{process.env.API_KEY}}

4. 提示变量：
   - {{?请输入用户名}} 执行时弹出输入框
   - 同一提示变量只弹一次
   - CLI/CI 环境下含提示变量的请求跳过并标记 skipped
   - 取消输入则不发送请求

5. 内置动态变量增强：
   - {{$timestamp}} 毫秒时间戳
   - {{$uuid}} UUID v4
   - {{$randomInt}} 0-9999 随机整数
   - {{$randomStr}} 8位随机字符串
   - {{$date}} YYYY-MM-DD
   - {{$randomEmail}} 随机邮箱
   - {{$randomPhone}} 随机手机号
   - {{$oauth2.xxx.access_token}} OAuth2 Token 引用

6. 环境颜色标识：每个环境可设置 color 字段，在环境选择器中显示

**验收标准**：
- [ ] 7 层变量优先级正确
- [ ] 对象/数组插值正常
- [ ] 提示变量在 TUI 中弹出输入框
- [ ] 提示变量在 CLI 中跳过
- [ ] 进程环境变量可读取
- [ ] 新增动态变量可用
- [ ] 环境颜色标识显示

**测试用例**：
```
test_variable_priority_runtime_highest
test_variable_priority_global_lowest
test_variable_object_interpolation
test_variable_array_interpolation
test_prompt_variable_tui_input
test_prompt_variable_cli_skip
test_prompt_variable_dedup
test_process_env_variable
test_builtin_random_email
test_builtin_random_phone
test_oauth2_token_variable
test_environment_color_display
```

**涉及文件**：models.rs, utils.rs, http_client.rs, app.rs, ui.rs, cli.rs

---

#### F11: API 文档生成

**用户故事**：作为开发者，我希望从集合自动生成 API 文档，不需要额外写文档工具。

**功能规格**：

1. 四级文档：Workspace / Collection / Folder / Request
2. 每级 docs 字段支持 Markdown
3. `helios doc generate "My API"` 生成文档
4. 输出格式：
   - Markdown（默认，适合 Git 仓库）
   - HTML（单页应用，暗色主题，适合分享）
5. `helios doc serve "My API" --port 8080` 本地启动文档服务器
6. 文档内容：
   - 请求名、方法、URL
   - 请求参数表
   - 请求头表
   - 请求体示例
   - 响应示例（从 Response Examples 取）
   - 认证说明
   - 自定义 Markdown 文档
7. 自动生成 Table of Contents
8. 支持自定义模板（.helios/doc-template.hbs）

**验收标准**：
- [ ] Markdown 文档生成正确
- [ ] HTML 文档美观可读
- [ ] 本地文档服务器正常启动
- [ ] 四级文档层级正确
- [ ] 响应示例嵌入文档

**测试用例**：
```
test_doc_generate_markdown
test_doc_generate_html
test_doc_serve_local_server
test_doc_collection_level
test_doc_folder_level
test_doc_request_level
test_doc_with_response_examples
test_doc_table_of_contents
test_doc_custom_template
```

**涉及文件**：docgen.rs(新), cli.rs, storage.rs

---

#### F12: 声明式脚本系统升级

**用户故事**：作为开发者，我希望用声明式语法定义请求前后的操作，而不是写 JavaScript。

**功能规格**：

1. 前置操作 (pre_request)：
   - set_header: 设置请求头
   - set_var: 设置变量
   - set_param: 设置查询参数
   - delay: 延迟毫秒数

2. 后置操作 (post_response)：
   - extract: 从响应提取变量
   - set_var: 设置运行时变量
   - assert: 断言（复用已有断言模块）

3. 请求链 (request chaining)：
   - run_request: 执行集合中的另一个请求
   - 条件执行：基于当前响应决定是否执行

4. 脚本执行流：
   - Sandwich 模式（默认）：集合Pre → 文件夹Pre → 请求Pre → 请求Post → 文件夹Post → 集合Post
   - Sequential 模式：集合Pre → 文件夹Pre → 请求Pre → 集合Post → 文件夹Post → 请求Post

5. 在 .helios.yml 中的 runtime 区域定义
6. TUI Script 标签页可视化编辑

**验收标准**：
- [ ] 前置操作正确执行
- [ ] 后置操作正确执行
- [ ] extract 从响应提取变量
- [ ] 请求链 run_request 正常
- [ ] Sandwich 和 Sequential 两种执行流正确
- [ ] TUI Script 标签页可编辑

**测试用例**：
```
test_pre_request_set_header
test_pre_request_set_var
test_pre_request_delay
test_post_response_extract_simple
test_post_response_extract_nested
test_post_response_assert
test_request_chain_run_request
test_script_flow_sandwich
test_script_flow_sequential
test_script_editor_in_tui
```

**涉及文件**：scripting.rs(重构), models.rs, http_client.rs, app.rs, ui.rs

---

### Phase 3：高级功能（P3，4周）

---

#### F13: Mock 服务器

**用户故事**：作为开发者，我希望快速启动一个 Mock 服务器来模拟 API，不依赖真实后端。

**功能规格**：

1. `helios mock start --port 8080` 启动本地 Mock 服务器
2. 从集合的请求 + 响应示例自动生成 Mock
3. 自定义 Mock 响应：
   ```yaml
   # mock-routes.yml
   - method: GET
     path: /users
     status: 200
     body: '{"users": []}'
     delay: 100    # 模拟延迟
   - method: POST
     path: /users
     status: 201
     body: '{"id": 1}'
   ```
4. `helios mock add "GET /users" --body '{"users":[]}' --status 200`
5. TUI 按 `M` 为当前请求快速添加 Mock
6. 支持动态 Mock（基于请求参数生成响应）
7. Mock 日志实时显示在 TUI

**测试用例**：
```
test_mock_server_start
test_mock_response_match_get
test_mock_response_match_post
test_mock_delay_simulation
test_mock_dynamic_response
test_mock_add_from_tui
test_mock_log_display
```

**涉及文件**：mock.rs(新), cli.rs, storage.rs

---

#### F14: 请求录制与回放 (VCR)

**用户故事**：作为开发者，我希望录制真实 API 响应，然后离线回放，用于稳定测试。

**功能规格**：

1. `helios record start` 开启录制模式
2. 录制的请求/响应对保存为 cassette（JSON 文件）
3. `helios record stop` 停止录制
4. `helios replay cassette.json` 从录制回放（不发真实请求）
5. `helios replay cassette.json --diff` 回放时对比实时响应与录制响应
6. 用于：离线测试、稳定 CI、性能对比基线
7. Cassette 文件存储在 .helios/cassettes/ 目录

**测试用例**：
```
test_record_cassette_create
test_record_request_response_pair
test_replay_no_real_request
test_replay_diff_mode
test_replay_cassette_expiry
```

**涉及文件**：vcr.rs(新), http_client.rs, cli.rs

---

#### F15: Breaking Changes 检测

**用户故事**：作为开发者，我希望在修改 API 后自动检测破坏性变更，避免影响下游消费者。

**功能规格**：

1. 对比两个版本（Git tag 或目录快照）的集合差异
2. 检测以下 Breaking Changes：
   - 删除端点
   - HTTP 方法变更
   - 必填参数新增
   - 响应字段删除
   - 响应字段类型变更
3. 输出兼容性报告：兼容 / 警告 / 破坏性变更
4. `helios breaking-check v1.0.0 v2.0.0`
5. 集成到 CI：`helios ci check --breaking`

**测试用例**：
```
test_breaking_removed_endpoint
test_breaking_method_change
test_breaking_new_required_param
test_breaking_response_field_removed
test_breaking_type_change
test_breaking_none_compatible
```

**涉及文件**：breaking.rs(新), diff.rs, cli.rs

---

#### F16: Cookie 管理

**用户故事**：作为开发者，我希望请求间的 Cookie 自动管理，支持查看和修改。

**功能规格**：

1. 全局 Cookie Jar（内存 + 持久化）
2. Response 面板新增 Cookies 标签页
3. 显示 Cookie 属性：Domain, Path, Key, Value, Expiration, Secure, HTTPOnly
4. 支持手动添加/修改/删除 Cookie
5. `helios cookie list` 列出所有 Cookie
6. `helios cookie add --domain example.com --key session --value abc123`
7. 集合级 Cookie 策略配置
8. Cookie 跟随请求自动发送

**测试用例**：
```
test_cookie_jar_persistence
test_cookie_auto_send_with_request
test_cookie_display_in_response
test_cookie_add_manual
test_cookie_delete
test_cookie_attributes
test_cookie_domain_matching
```

**涉及文件**：cookies.rs(新), http_client.rs, ui.rs, models.rs

---

#### F17: 响应示例保存

**用户故事**：作为开发者，我希望保存同一个请求的不同响应示例，用于文档和测试参考。

**功能规格**：

1. 执行请求后按 `E` 保存当前响应为 Example
2. Example 命名（如 "成功响应", "404错误"）
3. 每个 Example 存储：请求详情 + 响应数据 + 响应头
4. Example 列表显示在请求文件树下（子节点）
5. `helios example list "My API/login"`
6. 文档生成时自动嵌入响应示例
7. Mock 服务器可从 Example 生成 Mock 响应

**测试用例**：
```
test_example_save_response
test_example_list
test_example_load_and_compare
test_example_in_doc_generation
test_example_in_mock_server
test_example_as_sub_node_in_sidebar
```

**涉及文件**：models.rs, storage.rs, app.rs, ui.rs, docgen.rs, mock.rs

---

#### F18: 数据驱动测试

**用户故事**：作为开发者，我希望用 CSV/JSON 数据文件驱动集合运行，每行数据执行一次。

**功能规格**：

1. 支持 CSV 和 JSON 两种数据文件格式
2. CSV 列名 = 变量名，用 {{column_name}} 引用
3. JSON 数组格式，每个元素 = 一次迭代
4. `helios run --csv-file-path data.csv`
5. `helios run --json-file-path data.json`
6. Collection Runner API：
   - bru.runner.iterationData.get("key")
   - bru.runner.iterationIndex
   - bru.runner.totalIterations
7. `--iteration-count N` 重复运行 N 次
8. `--parallel` 并行执行

**CSV 示例**：
```csv
username,password,expected_status
admin,admin123,200
user,user123,200
guest,wrong,401
```

**测试用例**：
```
test_data_driven_csv_basic
test_data_driven_json_basic
test_data_driven_iteration_count
test_data_driven_parallel
test_data_driven_variable_substitution
```

**涉及文件**：models.rs, cli.rs, report.rs

---

#### F19: gRPC 支持

**用户故事**：作为开发者，我需要测试 gRPC 服务，不只有 REST。

**功能规格**：

1. 请求类型新增 gRPC
2. 支持 Proto 文件上传和解析
3. 支持 Server Reflection（无需 Proto 文件）
4. 支持 4 种流模式：Unary / Client Streaming / Server Streaming / Bidirectional Streaming
5. .helios.yml 中 type: grpc
6. TUI gRPC 请求界面：endpoint + method 选择 + 消息编辑
7. 流式消息实时显示

**测试用例**：
```
test_grpc_unary_request
test_grpc_server_streaming
test_grpc_client_streaming
test_grpc_bidirectional_streaming
test_grpc_proto_file_parse
test_grpc_server_reflection
```

**涉及文件**：grpc.rs(新), models.rs, http_client.rs, ui.rs

---

#### F20: WebSocket 支持

**用户故事**：作为开发者，我需要测试 WebSocket 实时通信接口。

**功能规格**：

1. 请求类型新增 websocket
2. 连接管理：连接 / 断开 / 重连
3. 消息类型：Text / JSON / Binary
4. 消息历史：时间戳 + 方向（发送/接收）+ 内容预览
5. .helios.yml 中 type: websocket
6. TUI WebSocket 界面：连接状态 + 消息编辑器 + 消息历史

**测试用例**：
```
test_websocket_connect
test_websocket_send_text
test_websocket_send_json
test_websocket_receive_message
test_websocket_disconnect
test_websocket_message_history
```

**涉及文件**：websocket.rs(新), models.rs, http_client.rs, ui.rs

---

#### F21: 代码生成器

**用户故事**：作为开发者，我希望能从请求生成各语言的代码片段，方便在项目中使用。

**功能规格**：

1. 支持生成 20+ 语言的代码：
   - curl, HTTPie, Python (requests), JavaScript (fetch/axios)
   - Java (OkHttp/HttpClient), Go (net/http), Rust (reqwest)
   - PHP (curl), Ruby (net/http), C# (HttpClient)
   - Swift, Kotlin, Dart 等
2. TUI 按 `Ctrl+G` 打开代码生成弹窗
3. 选择语言后显示代码，可复制
4. `helios codegen "My API/login" --lang python`
5. `helios codegen "My API/login" --lang curl` （已有 curl 导出功能的升级）

**测试用例**：
```
test_codegen_curl
test_codegen_python_requests
test_codegen_javascript_fetch
test_codegen_java_okhttp
test_codegen_go_net_http
test_codegen_rust_reqwest
```

**涉及文件**：codegen.rs(新), cli.rs, ui.rs

---

### Phase 4：生态建设（P4，持续）

---

#### F22: 插件系统

**用户故事**：作为开发者，我希望通过插件扩展 Helios 的功能，而不需要修改核心代码。

**功能规格**：

1. 插件类型：
   - Auth Provider（自定义认证方式）
   - Body Transformer（请求体转换）
   - Response Viewer（自定义响应渲染）
   - CLI Command（自定义命令）
2. 插件用 WASM 沙箱运行（安全隔离）
3. 插件 manifest：helios-plugin.toml
4. `helios plugin install <name>`
5. `helios plugin list`
6. `helios plugin uninstall <name>`
7. 插件目录：~/.helios/plugins/

**测试用例**：
```
test_plugin_load_wasm
test_plugin_auth_provider
test_plugin_body_transformer
test_plugin_response_viewer
test_plugin_sandbox_isolation
test_plugin_install_uninstall
```

**涉及文件**：plugin.rs(新), plugin_loader.rs(新), cli.rs

---

#### F23: Webhook 调试隧道

**用户故事**：作为开发者，我需要一个公网隧道来接收和调试 Webhook 回调。

**功能规格**：

1. `helios tunnel start --port 8080` 创建公网隧道
2. 接收的请求自动记录到历史
3. 支持自动回复（200 OK 或自定义响应）
4. 隧道 URL 自动设为 {{tunnel_url}} 变量
5. 支持隧道认证（Bearer Token）
6. TUI 中实时显示接收到的请求

**测试用例**：
```
test_tunnel_start
test_tunnel_webhook_capture
test_tunnel_auto_respond
test_tunnel_url_variable
test_tunnel_auth
```

**涉及文件**：tunnel.rs(新), cli.rs

---

## 五、非功能需求

### 5.1 性能

| 指标 | 目标 |
|------|------|
| 冷启动时间 | < 100ms |
| 发送请求延迟 | < 50ms (不含网络时间) |
| 加载1000个请求的集合 | < 500ms |
| 内存占用 | < 20MB (空载) |
| 文件监听延迟 | < 200ms |

### 5.2 兼容性

| 平台 | 要求 |
|------|------|
| macOS 12+ | 主要支持 |
| Linux (x86_64/ARM64) | 完整支持 |
| SSH 远程终端 | 核心功能可用 |
| Windows Terminal | 基础支持 |

### 5.3 安全

| 要求 | 说明 |
|------|------|
| 密钥加密存储 | AES-256-GCM |
| 无云端数据传输 | 所有数据本地存储 |
| .helios/ 目录 gitignore 默认 | 防止意外提交密钥 |
| 敏感信息检测 | git check 扫描 |

### 5.4 可访问性

| 要求 | 说明 |
|------|------|
| 键盘完全可操作 | 无鼠标依赖 |
| 高对比度主题 | 暗色/亮色双主题 |
| 屏幕阅读器 | 结构化输出 |
| 可配置快捷键 | .helios/keybindings.yml |

---

## 六、版本规划

```
v3.0 (Phase 0) ─ 基础设施
  F01 .helios.yml 文件格式
  F02 集合即目录
  F03 密钥保险箱
  里程碑: 存储100%文件化，不再依赖 data.json

v3.1 (Phase 1) ─ 开发者体验
  F04 智能补全
  F05 请求标签
  F06 性能剖析器
  F07 CI 配置文件
  F08 7种认证协议
  里程碑: DX 对齐 Bruno 核心体验

v3.2 (Phase 2) ─ 协作与安全
  F09 Git 感知
  F10 7层变量系统
  F11 API 文档生成
  F12 声明式脚本升级
  里程碑: Git-Native 协作闭环

v3.3 (Phase 3) ─ 高级功能
  F13 Mock 服务器
  F14 VCR 录制回放
  F15 Breaking Changes 检测
  F16 Cookie 管理
  F17 响应示例
  F18 数据驱动测试
  F19 gRPC 支持
  F20 WebSocket 支持
  F21 代码生成器
  里程碑: API 全生命周期覆盖

v4.0 (Phase 4) ─ 生态建设
  F22 WASM 插件系统
  F23 Webhook 隧道
  里程碑: 可扩展生态
```

---

## 七、成功指标

### 7.1 量化指标

| 指标 | v3.0 目标 | v3.3 目标 |
|------|-----------|-----------|
| GitHub Star | 500 | 5000 |
| 月活用户 | 100 | 2000 |
| CI 集成率 | 5% | 30% |
| 文件格式采用率 | 60% | 95% |
| 密钥保险箱使用率 | 20% | 60% |

### 7.2 定性指标

1. Hacker News 首页推荐
2. 至少 3 个 Bruno 用户公开迁移到 Helios
3. Rust 社区认可（r/rust 推荐帖）
4. 至少 1 家公司在 CI 中使用 Helios

---

## 附录 A：Bruno vs Helios 功能差距速查表

| 功能 | Bruno | Helios 现状 | PRD Feature |
|------|-------|-------------|-------------|
| REST 请求 | 完整 | 完整 | - |
| GraphQL | Builder+变量 | 基础Body | F08 |
| gRPC | Proto+4种流 | 无 | F19 |
| WebSocket | 连接+消息历史 | 无 | F20 |
| SOAP/WSDL | WSDL导入 | 无 | 不在PRD内 |
| Basic Auth | 有 | 有 | - |
| Bearer Auth | 有 | 有 | - |
| OAuth2 | 4种Grant+自动刷新 | 无 | F08 |
| AWS SigV4 | 有 | 无 | F08 |
| Digest Auth | 有 | 无 | F08 |
| NTLM Auth | 有 | 无 | 不在PRD内 |
| Cookie 管理 | 完整 | 无 | F16 |
| 代码生成 | 35+语言 | curl导出 | F21 |
| 7层变量 | 完整 | 2层 | F10 |
| 提示变量 | 有 | 无 | F10 |
| 集合变量 | 有 | 无 | F10 |
| 文件夹变量 | 有 | 无 | F10 |
| 对象/数组插值 | 有 | 无 | F10 |
| JS 脚本 | V8引擎 | 声明式DSL | F12 |
| 请求链 | bru.runRequest | 简单依赖链 | F12 |
| Chai 断言 | 完整 | 6种操作符 | F12 |
| 脚本执行流 | Sandwich/Sequential | 无 | F12 |
| Secret Variables | OS加密+AES256 | 无 | F03 |
| AWS/Azure/Vault | 3种Secret Manager | 无 | 不在PRD内 |
| Secret 脱敏 | 报告中遮蔽 | 无 | F03 |
| 客户端证书 | PEM/PKCS12 | 无 | F08 |
| 集合文件系统 | .bru + YAML | data.json | F01+F02 |
| Git GUI操作 | Pull/Push/Branch | 无 | F09 |
| Git 状态标记 | 有 | 无 | F09 |
| .gitignore 排除 | 无 | 无 | F09 |
| API 文档 | 4级+自动生成 | 无 | F11 |
| OpenAPI Sync | 远程规范同步 | 无 | 不在PRD内 |
| DevTools | Console/Network/Perf/Terminal | 无 | F06 |
| Timeline | 3标签 | 无 | F06 |
| 响应示例 | 保存多次 | 无 | F17 |
| CLI 报告 | JSON/JUnit/HTML | JSON | F07 |
| 数据驱动测试 | CSV+JSON | 无 | F18 |
| 标签过滤运行 | --tags/--exclude-tags | 无 | F05 |
| 并行执行 | --parallel | 无 | F18 |
| VS Code 扩展 | 完整 | 无 | 不在PRD内 |
| AI Agent | 4种 | 无 | 不在PRD内 |
| 转换器 | 4种 | 2种 | F01迁移兼容 |
| Mock 服务器 | 无 | 无 | F13 |
| VCR 录制回放 | 无 | 无 | F14 |
| Breaking Changes | 无 | 无 | F15 |
| 性能剖析 | Timeline | 无 | F06 |
| 插件系统 | 无 | 无 | F22 |
| Webhook 隧道 | 无 | 无 | F23 |

---

## 附录 B：.helios.yml 与 Bruno .bru 格式对比

### Bruno .bru 格式

```
meta {
    name: Create User
    type: http
    seq: 1
}

post {
    url: {{base_url}}/users
    body: json
    auth: bearer
}

headers {
    Content-Type: application/json
    Authorization: Bearer {{auth_token}}
}

body:json {
    {
        "name": "John",
        "email": "john@example.com"
    }
}

script:post-response {
    bru.setVar('user_id', res.body.id);
}

tests {
    test("status is 201", function() {
        expect(res.status).to.equal(201);
    });
}
```

### Helios .helios.yml 格式

```yaml
info:
  name: Create User
  type: http
  seq: 1

http:
  method: POST
  url: "{{base_url}}/users"
  headers:
    - key: Content-Type
      value: application/json
      enabled: true
    - key: Authorization
      value: "Bearer {{auth_token}}"
      enabled: true
  body:
    type: json
    content: |
      {
        "name": "John",
        "email": "john@example.com"
      }
  auth:
    type: bearer
    token: "{{auth_token}}"

runtime:
  post_response:
    - action: extract
      var_name: user_id
      json_path: "$.id"
  tests:
    - name: status is 201
      assert: status
      operator: equals
      expected: "201"
```

### 设计选择说明

| 方面 | Bruno .bru | Helios .helios.yml | 理由 |
|------|-----------|-------------------|------|
| 格式 | 自定义 DSL | 标准 YAML | YAML 有通用工具链(yamllint/prettier/IDE高亮) |
| 语法 | 大括号分块 | YAML 缩进 | YAML 更符合 IaC 生态习惯 |
| 脚本 | JavaScript | 声明式 action 列表 | TUI 不适合写 JS，声明式更轻量 |
| 测试 | Chai expect | 结构化断言 | 声明式断言可由 TUI 可视化编辑 |
| 扩展性 | 新增 tag | 新增 YAML key | YAML 扩展更自然 |
| 学习曲线 | 需学 .bru 语法 | YAML 通用知识 | 降低入门门槛 |

---

## 附录 C：TUI 快捷键完整规划

### 全局快捷键

| 按键 | 功能 | 备注 |
|------|------|------|
| Ctrl+Q / Ctrl+C | 退出 | 已有 |
| m + 1/2/3/4 | 切换面板 | 已有 |
| Tab / Shift+Tab | 循环面板 | 已有 |
| / | 打开搜索 | 已有 |
| Ctrl+S | 保存请求 | 已有 |
| Ctrl+Z | 撤销编辑 | 新增 |
| Ctrl+D | 切换 DevTools | 新增 |
| Ctrl+G | 代码生成 | 新增 |
| Ctrl+E | 导出集合 | 已有 |
| Ctrl+T | 新建标签页 | 已有 |
| Ctrl+W | 关闭标签页 | 已有 |
| Ctrl+1..9 | 切换标签页 | 已有 |
| G | 查看 Git 状态 | 新增 |
| ? | 帮助 | 新增 |

### 侧边栏快捷键

| 按键 | 功能 | 备注 |
|------|------|------|
| 1/2/3/4 | 切换标签页(Coll/Env/Hist/Git) | 扩展 |
| ↑/↓ | 导航 | 已有 |
| →/← | 展开/折叠 | 已有 |
| Enter | 加载请求 | 已有 |
| Ctrl+N | 新建集合 | 已有 |
| Ctrl+R | 新建请求 | 已有 |
| d | 删除 | 已有 |
| T | 标签过滤模式 | 新增 |

### 请求配置快捷键

| 按键 | 功能 | 备注 |
|------|------|------|
| p/h/b/a/s/t/v/d | 切换标签页 | 扩展 |
| n | 新增行 | 已有 |
| d | 删除行 | 已有 |
| e | 编辑 Key | 已有 |
| v | 编辑 Value | 已有 |
| Space | 启用/禁用 | 已有 |

### 响应面板快捷键

| 按键 | 功能 | 备注 |
|------|------|------|
| b/h/t/c/e | 切换标签页(Body/Headers/Timeline/Cookies/Examples) | 扩展 |
| ↑/↓ | 滚动 | 已有 |
| PgUp/PgDn | 翻页 | 已有 |
| g/G | 跳到顶部/底部 | 已有 |
| y | 复制 | 已有 |
| E | 保存为示例 | 新增 |
| D | 对比 Diff | 已有 |

---

## 附录 D：与 Bruno OpenCollection YAML 对齐计划

Bruno v3 推出了 OpenCollection YAML 规范（https://docs.usebruno.com/opencollection-yaml/overview.md），这是一个开放标准。

Helios 的 .helios.yml 应尽可能与 OpenCollection YAML 对齐：

| OpenCollection YAML 字段 | Helios 对应 | 兼容性 |
|--------------------------|------------|--------|
| info.name | info.name | 完全兼容 |
| info.type | info.type | 扩展(grpc/websocket) |
| info.seq | info.seq | 完全兼容 |
| info.tags | info.tags | 完全兼容 |
| http.method | http.method | 完全兼容 |
| http.params | http.params | 完全兼容 |
| http.headers | http.headers | 完全兼容 |
| http.body | http.body | 完全兼容 |
| http.auth | http.auth | 扩展(更多auth类型) |
| runtime.scripts | runtime.pre_request/post_response | 结构不同(声明式vs JS) |
| runtime.assertions | runtime.tests | 结构不同(声明式vs JS) |
| settings.* | settings.* | 完全兼容 |
| docs | docs | 完全兼容 |

**兼容策略**：
1. Helios 能读取 OpenCollection YAML 文件（忽略不认识的字段）
2. Helios 的 .helios.yml 是 OpenCollection YAML 的超集
3. 导出时可选择 OpenCollection 兼容模式（去除声明式脚本，转注释）
4. `helios import --format opencollection <dir>` 导入 Bruno YAML 集合
