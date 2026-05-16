# Helios V3 产品路线图 — 从 API 工具到 API 开发平台

> 以 GitLab 产品经理视角，对标 Bruno(43.8k star)、HTTPie(38.1k star)、Hurl(18.9k star)、Postman、Insomnia，
> 从设计理念和用户体验角度，定义 Helios 的差异化竞争力和下一阶段演进方向。

---

## 一、现状盘点

Helios 已有 27 个 feature，覆盖：

| 领域 | 已有功能 |
|------|---------|
| 请求构建 | REST/GraphQL/multipart/前置脚本/Body模板/变量替换/curl导入 |
| 响应分析 | 断言/提取变量/JSONPath查询/响应对比/历史对比 |
| 环境变量 | 全局变量/内置动态变量/多环境 |
| 测试流程 | 场景编排/依赖链/测试报告 |
| 数据格式 | OpenAPI导入/HAR导入/Postman导出/Helios JSON |
| 交互体验 | 搜索/标签页/请求克隆/curl导出 |

**短板**：缺协作、缺安全、缺文档、缺CI/CD、缺插件生态、缺Git友好工作流。

---

## 二、设计理念升级

### 理念1：Git-First, File-Based（致敬 Hurl + Bruno）

Bruno 的核心差异化就是"文件优先、Git 友好"。Postman 用云同步，Bruno 用文件系统。
Helios 应更进一步：请求定义即代码，天然可 diff、可 review、可 CI。

### 理念2：零摩擦入链（致敬 HTTPie）

HTTPie 的核心理念是"人类可读的 HTTP"。Helios 应做到：
- 从想法到请求 < 3 秒
- 从请求到断言 < 5 秒
- 从断言到CI流水线 < 30 秒

### 理念3：终端原生协作（对标 GitLab）

不是把 Postman 搬到终端，而是重新定义终端里的 API 协作方式。
终端用户的协作方式是 Git + 代码 Review，不是共享链接。

### 理念4：可编程但声明式（致敬 Hurl + Terraform）

不引入 JavaScript 引擎（不像 Postman 的 Pre-request Script），
但通过声明式 DSL 实现 90% 的编程需求。

---

## 三、功能规划（6 大赛道，36 个新 feature）

### 赛道G：Git-First 工作流（8个） — 核心差异化

#### G1: .hurl 文件格式支持
**对标**: Hurl 的纯文本 .hurl 格式
**设计**:
- 定义 .helios 文件格式（YAML/纯文本混合）
- 一个文件 = 一个请求，一个目录 = 一个集合
- 纯文本可 diff，天然适合 Git
- `helios init my-project` 生成项目骨架
- 与现有 data.json 双向转换
**文件**: helios_format.rs, cli.rs, storage.rs
**测试**: test_helios_format_parse, test_helios_format_roundtrip, test_helios_to_json_migration

#### G2: 集合即目录
**对标**: Bruno 的文件系统集合
**设计**:
- 集合映射为文件系统目录
- 请求映射为 .helios 文件
- 环境配置映射为 .helios.env 文件
- 目录结构即集合层级
- `helios sync` 检测文件变更并更新内存
**文件**: storage.rs, helios_format.rs
**测试**: test_collection_as_directory, test_request_as_file, test_env_as_file

#### G3: Git 感知与变更追踪
**对标**: GitLab 的 MR diff view
**设计**:
- `helios git status` 显示请求变更（新增/修改/删除）
- `helios git diff` 显示请求前后差异（复用 diff 模块）
- `helios git log` 显示请求修改历史
- 在 TUI 中按 `G` 查看当前请求的 Git 状态
- 检测 .helios 文件的 git blame 信息
**文件**: git_integration.rs (新模块)
**测试**: test_git_status_new, test_git_status_modified, test_git_diff_request

#### G4: .heliosignore 与敏感数据排除
**对标**: .gitignore
**设计**:
- .heliosignore 文件排除特定请求/集合
- 标记包含密钥的请求为 secret，不提交到 Git
- `helios git check` 检测潜在敏感信息泄露
- 支持正则匹配和目录排除
**文件**: git_integration.rs
**测试**: test_heliosignore_patterns, test_secret_detection, test_git_check

#### G5: 请求快照与回滚
**对标**: Git stash / Git reset
**设计**:
- 发送请求前自动创建快照
- `helios snapshot list` 查看快照
- `helios snapshot restore <id>` 回滚到指定快照
- TUI 按 `Ctrl+Z` 撤销请求编辑
**文件**: snapshot.rs (新模块), storage.rs
**测试**: test_snapshot_create, test_snapshot_restore, test_snapshot_auto_before_send

#### G6: 集合版本标签
**对标**: Git tags / Semantic Versioning
**设计**:
- `helios tag v1.0.0` 为集合打版本标签
- 标签存储在 .helios-meta 文件中
- `helios tag list` 查看所有标签
- `helios tag diff v1.0.0 v1.1.0` 对比版本差异
**文件**: git_integration.rs, storage.rs
**测试**: test_tag_create, test_tag_list, test_tag_diff

#### G7: 变更评审工作流
**对标**: GitLab MR / GitHub PR
**设计**:
- `helios review create` 生成变更集描述（哪些请求新增/修改）
- `helios review diff` 生成人类可读的变更摘要
- 输出 Markdown 格式，可直接粘贴到 PR 描述
- 与 git hook 集成，pre-commit 时自动检查请求变更
**文件**: review.rs (新模块, 与 report.rs 不同)
**测试**: test_review_create, test_review_diff_markdown, test_pre_commit_hook

#### G8: 多项目工作区
**对标**: VS Code Workspace / IntelliJ Project
**设计**:
- `helios workspace add ~/project-a ~/project-b` 管理多项目
- `helios workspace switch project-a` 切换上下文
- 每个工作区独立的集合、环境、配置
- TUI 顶部显示当前工作区名称
**文件**: workspace.rs (新模块), storage.rs, cli.rs
**测试**: test_workspace_create, test_workspace_switch, test_workspace_isolation

---

### 赛道H：安全与认证（5个） — 企业级刚需

#### H9: 密钥保险箱
**对标**: 1Password CLI / Vault / Bruno 的 .env 文件
**设计**:
- 新增 `src/vault.rs` 模块
- 密钥存储在系统 Keychain (macOS) 或加密文件
- 变量引用 `{{vault:api_key}}` 自动从保险箱取值
- `helios vault set api_key sk-xxx`
- `helios vault list` 列出密钥名（不显示值）
- 密钥永不写入 data.json 或 .helios 文件
**文件**: vault.rs (新模块), models.rs, storage.rs
**测试**: test_vault_set_get, test_vault_not_in_export, test_vault_keychain_integration

#### H10: OAuth 2.0 流程
**对标**: Postman OAuth 2.0 / Insomnia
**设计**:
- Auth 类型新增 OAuth2 变体
- 支持 Authorization Code / Client Credentials / Password / Implicit
- 自动刷新 token（refresh_token 自动续期）
- token 过期前 60 秒自动刷新
- 内置本地回调服务器（Authorization Code 流程）
**文件**: auth.rs (新模块), http_client.rs, models.rs
**测试**: test_oauth2_client_credentials, test_oauth2_token_refresh, test_oauth2_auto_inject

#### H11: mTLS 双向认证
**对标**: Postman / curl --cert
**设计**:
- Collection 级别配置客户端证书
- 支持 PEM / PKCS12 格式
- `helios cert add --cert client.pem --key key.pem`
- 请求时自动附加客户端证书
**文件**: auth.rs, http_client.rs
**测试**: test_mtls_pem, test_mtls_pkcs12, test_mtls_per_collection

#### H12: 签名请求（AWS SigV4 / HMAC）
**对标**: Postman AWS Signature / HTTPie Auth
**设计**:
- Auth 类型新增 AwsSigV4 / HmacSha256
- 自动计算签名并注入 Authorization header
- 支持 AWS Access Key / Secret Key 从 vault 引用
- `helios auth aws --region us-east-1 --service execute-api`
**文件**: auth.rs
**测试**: test_aws_sigv4_sign, test_hmac_sign, test_sigv4_with_vault_credentials

#### H13: 敏感数据脱敏
**对标**: GitLab secrets masking
**设计**:
- 响应显示时自动遮蔽 credit_card / ssn / api_key 等模式
- `helios config set masking enabled`
- 自定义脱敏规则（正则匹配 → 替换为 ***）
- 导出报告时同样脱敏
**文件**: masking.rs (新模块), ui.rs, report.rs
**测试**: test_mask_credit_card, test_mask_custom_pattern, test_mask_in_export

---

### 赛道I：API 生命周期管理（6个） — 超越测试工具

#### I14: API 文档生成
**对标**: Postman 文档 / Stoplight / Swagger UI
**设计**:
- 从集合自动生成 Markdown / HTML 文档
- 包含：端点、方法、参数、请求示例、响应示例
- `helios doc generate "My API" --format markdown`
- `helios doc serve "My API"` 本地启动文档服务器
- 支持自定义文档模板
**文件**: docgen.rs (新模块), cli.rs
**测试**: test_docgen_markdown, test_docgen_html, test_docgen_with_examples

#### I15: API 变更检测（Breaking Changes）
**对标**: Stoplight / oasdiff
**设计**:
- 对比两个版本的集合/请求差异
- 检测 Breaking Changes：删除端点、必填字段新增、类型变更、响应结构变更
- `helios breaking-check v1.0.0 v2.0.0`
- 输出兼容性报告（兼容/警告/破坏性变更）
**文件**: breaking.rs (新模块), diff.rs
**测试**: test_breaking_removed_endpoint, test_breaking_new_required_field, test_breaking_type_change

#### I16: Mock 服务器
**对标**: Postman Mock Server / Prism
**设计**:
- `helios mock start --port 8080` 启动本地 Mock 服务器
- 从集合的请求/响应示例生成 Mock 响应
- 支持延迟模拟、状态码切换
- `helios mock add "GET /users" --response '{"users":[]}' --status 200`
- TUI 按 `M` 快速为当前请求添加 Mock 响应
**文件**: mock.rs (新模块), cli.rs
**测试**: test_mock_server_start, test_mock_response_match, test_mock_delay

#### I17: 契约测试（Contract Testing）
**对标**: Pact / Spring Cloud Contract
**设计**:
- 从断言自动生成消费者契约
- `helios contract export "My API" --format pact`
- `helios contract verify pact.json` 验证提供者是否满足契约
- 契约存储在 .helios/contracts/ 目录
**文件**: contract.rs (新模块), assertions.rs
**测试**: test_contract_export_pact, test_contract_verify_pass, test_contract_verify_fail

#### I18: API 健康监控
**对标**: Postman Monitoring / UptimeRobot
**设计**:
- `helios monitor add "Health Check" --interval 5m --collection "My API"`
- 定时执行集合，记录响应时间和状态码
- `helios monitor status` 查看监控状态
- 响应超阈值时终端通知（macOS notification）
- 历史数据持久化，可生成 SLA 报告
**文件**: monitor.rs (新模块), cli.rs
**测试**: test_monitor_create, test_monitor_threshold_alert, test_monitor_sla_report

#### I19: 请求录制与回放
**对标**: VCR (Ruby) / Polly.JS / go-vcr
**设计**:
- `helios record start` 开启录制模式
- 录制的请求/响应对保存为 cassette 文件
- `helios replay cassette.json` 回放（不发真实请求）
- 用于离线测试、稳定 CI、性能对比
- `helios replay --diff` 回放时对比实时响应与录制响应的差异
**文件**: vcr.rs (新模块), http_client.rs
**测试**: test_record_cassette, test_replay_from_cassette, test_replay_diff_mode

---

### 赛道J：开发者体验 DX（9个） — 决定留存率

#### J20: 智能补全引擎
**对标**: HTTPie 的直觉式语法 / IDE 自动补全
**设计**:
- URL 栏输入时自动补全（历史URL + 集合URL）
- Header 值补全（Content-Type → application/json 等）
- 变量名补全（{{}} 内自动列出可用变量）
- JSON Body 字段补全（基于上次响应 schema）
- Shell 补全脚本生成（bash/zsh/fish）
**文件**: completion.rs (新模块), ui.rs, events.rs
**测试**: test_complete_url, test_complete_header, test_complete_variable

#### J21: 请求链可视化
**对标**: Postman Collection Runner / GitLab CI Pipeline
**设计**:
- TUI 新增 Pipeline 面板（按 `P` 切换）
- 可视化展示请求依赖链（DAG 图）
- 每个节点显示请求名、状态（pending/running/pass/fail）
- 连线显示变量传递关系
- 失败节点标红，影响链路高亮
**文件**: pipeline.rs (新模块), ui.rs
**测试**: test_pipeline_dag_render, test_pipeline_variable_edge, test_pipeline_failure_highlight

#### J22: 请求书签与快捷键
**对标**: IDE Bookmarks / Vim marks
**设计**:
- `m+a..z` 为请求设置标记（最多 26 个）
- `'a..z` 跳转到标记的请求
- `helios bookmark list` 列出所有书签
- 书签持久化到 .helios-bookmarks
**文件**: bookmarks.rs (新模块), app.rs
**测试**: test_bookmark_set, test_bookmark_jump, test_bookmark_persist

#### J23: 请求模板库
**对标**: Postman Templates / GitHub Templates
**设计**:
- 内置常用 API 模板（OAuth login, Pagination, File Upload, Webhook）
- `helios template list` 查看可用模板
- `helios template use oauth-login` 从模板创建请求
- 用户自定义模板保存到 .helios/templates/
- TUI 按 `Ctrl+T` 后可选"从模板创建"
**文件**: templates.rs (新模块), storage.rs
**测试**: test_template_builtin_oauth, test_template_custom, test_template_apply

#### J24: 深链接与 CLI 桥接
**对标**: VS Code URI handler / xdg-open
**设计**:
- `helios://request/My%20API/login` 深链接打开指定请求
- `helios open "My API/login"` CLI 快速打开
- 支持从浏览器跳转到 Helios TUI
- 与 macOS URL Scheme 注册集成
**文件**: deeplink.rs (新模块), cli.rs
**测试**: test_deeplink_parse, test_deeplink_open_request, test_deeplink_url_scheme

#### J25: 请求注释与协作备注
**对标**: GitLab MR Comments / Google Docs Comments
**设计**:
- 请求新增 `comments: Vec<Comment>` 字段
- Comment { author: String, content: String, timestamp: i64, resolved: bool }
- TUI 按 `C` 查看/添加注释
- 注释存储在 .helios 文件中，可 Git 追踪
- `helios comment add "My API/login" "Token过期需要刷新逻辑"`
**文件**: models.rs, comments.rs (新模块)
**测试**: test_comment_add, test_comment_resolve, test_comment_in_helios_file

#### J26: 批量操作
**对标**: IDE Refactor / sed 批量替换
**设计**:
- `helios batch rename --from "v1" --to "v2"` 批量重命名
- `helios batch replace-url --from "api.dev" --to "api.prod"` 批量替换 URL
- `helios batch add-header "X-API-Version:2"` 批量添加 Header
- `helios batch export --filter "tag:smoke"` 按标签筛选导出
**文件**: batch.rs (新模块), cli.rs
**测试**: test_batch_rename, test_batch_replace_url, test_batch_add_header

#### J27: 请求标签与分类
**对标**: GitLab Labels / GitHub Topics
**设计**:
- Request 新增 `tags: Vec<String>` 字段
- `helios tag add "My API/login" smoke auth`
- TUI 侧边栏支持按标签过滤
- 标签带颜色（可自定义）
- `helios tag list` 查看所有标签及关联请求数
**文件**: models.rs, tags.rs (新模块), app.rs
**测试**: test_tag_add, test_tag_filter, test_tag_color

#### J28: 性能剖析器
**对标**: Chrome DevTools Network / curl -w
**设计**:
- 每次请求记录详细时序：DNS、TCP连接、TLS握手、TTFB、内容传输
- 复用 reqwest 的连接池状态信息
- TUI Response 面板新增 Timing tab
- `helios perf "My API"` 输出性能报告
- 对比同一请求在不同环境/时间的性能差异
**文件**: perf.rs (新模块), http_client.rs, ui.rs
**测试**: test_perf_timing_record, test_perf_compare, test_perf_threshold

---

### 赛道K：CI/CD 集成（4个） — DevOps 闭环

#### K29: .helios-ci.yml 配置文件
**对标**: GitLab CI .gitlab-ci.yml / GitHub Actions
**设计**:
- 项目根目录放置 .helios-ci.yml
- 定义：测试集合、断言阈值、环境、通知
- `helios ci run` 执行 CI 配置
- `helios ci check` 预检（语法验证、断言覆盖）
- 退出码语义化：0=全部通过, 1=断言失败, 2=请求错误
**文件**: ci_config.rs (新模块), cli.rs
**测试**: test_ci_config_parse, test_ci_config_run, test_ci_exit_codes

#### K30: GitHub Actions / GitLab CI 集成
**对标**: Hurl 的 CI/CD 集成
**设计**:
- `helios ci scaffold github` 生成 GitHub Actions workflow
- `helios ci scaffold gitlab` 生成 .gitlab-ci.yml
- Docker 镜像发布（helios:latest 用于 CI）
- 支持 --junit-output 生成 JUnit XML（CI 兼容格式）
- 支持 --json-output 生成机器可读结果
**文件**: ci_scaffold.rs (新模块), cli.rs
**测试**: test_scaffold_github_actions, test_scaffold_gitlab_ci, test_junit_output

#### K31: 滑动窗口回归测试
**对标**: GitLab Merge Train / Postman Scheduled Runs
**设计**:
- `helios regress --baseline v1.0.0` 对比基线版本
- 自动运行基线中的所有请求，对比响应差异
- 响应体 schema 对比（忽略动态值，对比结构）
- `helios regress --watch` 文件变更时自动回归
**文件**: regression.rs (新模块), cli.rs
**测试**: test_regress_baseline, test_regress_schema_diff, test_regress_watch

#### K32: 测试覆盖率报告
**对标**: Istanbul / gcov / GitLab Code Coverage
**设计**:
- `helios coverage` 计算集合中请求的断言覆盖率
- 覆盖维度：状态码断言、Body 断言、Header 断言
- 输出覆盖率百分比和未覆盖的请求列表
- `helios coverage --threshold 80` 低于阈值返回非零退出码
- 与 CI 集成，PR 中自动评论覆盖率变化
**文件**: coverage.rs (新模块), report.rs
**测试**: test_coverage_calculate, test_coverage_threshold, test_coverage_uncovered_list

---

### 赛道L：插件与生态（4个） — 长期护城河

#### L33: 插件系统
**对标**: HTTPie Plugins / Postman Interceptor
**设计**:
- `helios plugin install helios-plugin-jwt`
- 插件类型：Auth Provider / Body Transformer / Response Viewer / CLI Command
- 插件用 WASM 沙箱运行（安全隔离）
- `~/.helios/plugins/` 目录管理
- 插件 manifest: helios-plugin.toml
**文件**: plugin.rs (新模块), plugin_loader.rs
**测试**: test_plugin_load, test_plugin_auth_provider, test_plugin_wasm_sandbox

#### L34: 自定义响应渲染器
**对标**: Postman Visualizer / HTTPie --print
**设计**:
- 内置渲染器：JSON Tree、Raw、Preview（HTML渲染）、Hex（二进制）
- 可切换渲染器（Response 面板按 `R`）
- 图片响应显示为终端图片（Sixel/iTerm2 协议）
- 自定义渲染器通过插件注册
**文件**: renderers.rs (新模块), ui.rs
**测试**: test_renderer_json_tree, test_renderer_image_sixel, test_renderer_custom_plugin

#### L35: Webhook 调试隧道
**对标**: ngrok / Postman Webhook Tunnel / smee.io
**设计**:
- `helios tunnel start --port 8080` 创建公网隧道
- 接收的 Webhook 请求自动记录到历史
- 支持自动回复（200 OK / 自定义响应）
- 隧道 URL 自动设为环境变量 {{tunnel_url}}
**文件**: tunnel.rs (新模块), cli.rs
**测试**: test_tunnel_start, test_tunnel_webhook_capture, test_tunnel_auto_respond

#### L36: API 市场（Helios Hub）
**对标**: Postman API Network / RapidAPI
**设计**:
- `helios hub search stripe` 搜索公开 API 集合
- `helios hub install stripe-payments` 安装到本地
- 用户可发布自己的集合到 Hub
- Hub 元数据存储在 GitHub Repo（helios-hub/hub）
- 离线模式：缓存已安装的集合
**文件**: hub.rs (新模块), cli.rs
**测试**: test_hub_search, test_hub_install, test_hub_offline_cache

---

## 四、优先级排序（RICE 评分法）

| ID | Feature | Reach | Impact | Confidence | Effort | RICE Score | Phase |
|----|---------|-------|--------|------------|--------|------------|-------|
| G1 | .helios文件格式 | 9 | 10 | 8 | 5 | 144 | P0 |
| G2 | 集合即目录 | 9 | 9 | 8 | 4 | 162 | P0 |
| H9 | 密钥保险箱 | 8 | 9 | 9 | 4 | 162 | P0 |
| J20 | 智能补全 | 9 | 8 | 7 | 4 | 126 | P0 |
| K29 | CI配置文件 | 7 | 8 | 8 | 3 | 149 | P0 |
| H10 | OAuth 2.0 | 8 | 8 | 7 | 5 | 90 | P1 |
| G3 | Git感知 | 7 | 7 | 8 | 4 | 98 | P1 |
| I16 | Mock服务器 | 7 | 8 | 6 | 6 | 56 | P1 |
| J28 | 性能剖析 | 8 | 6 | 8 | 3 | 128 | P1 |
| J27 | 请求标签 | 8 | 5 | 9 | 2 | 180 | P1 |
| I14 | 文档生成 | 6 | 7 | 7 | 5 | 59 | P1 |
| K30 | CI脚手架 | 6 | 7 | 8 | 3 | 112 | P1 |
| G4 | .heliosignore | 6 | 5 | 8 | 2 | 120 | P1 |
| G5 | 请求快照 | 5 | 5 | 7 | 3 | 58 | P2 |
| H12 | 签名请求 | 5 | 6 | 6 | 4 | 45 | P2 |
| J21 | 请求链可视化 | 5 | 6 | 5 | 5 | 30 | P2 |
| J23 | 请求模板库 | 6 | 5 | 7 | 3 | 70 | P2 |
| J26 | 批量操作 | 6 | 4 | 7 | 3 | 56 | P2 |
| K31 | 回归测试 | 5 | 6 | 6 | 5 | 36 | P2 |
| I15 | 变更检测 | 5 | 6 | 5 | 5 | 30 | P2 |
| K32 | 覆盖率报告 | 5 | 5 | 6 | 4 | 38 | P2 |
| G6 | 版本标签 | 4 | 4 | 7 | 3 | 37 | P3 |
| G7 | 变更评审 | 4 | 5 | 5 | 4 | 25 | P3 |
| G8 | 多项目工作区 | 4 | 5 | 5 | 5 | 20 | P3 |
| H11 | mTLS | 3 | 5 | 6 | 4 | 23 | P3 |
| H13 | 敏感数据脱敏 | 5 | 4 | 6 | 3 | 40 | P3 |
| I17 | 契约测试 | 3 | 5 | 4 | 6 | 10 | P3 |
| I18 | 健康监控 | 3 | 4 | 5 | 5 | 12 | P3 |
| I19 | 录制回放 | 4 | 4 | 5 | 5 | 16 | P3 |
| J22 | 请求书签 | 5 | 3 | 7 | 2 | 53 | P2 |
| J24 | 深链接 | 3 | 3 | 5 | 3 | 15 | P3 |
| J25 | 请求注释 | 4 | 3 | 5 | 3 | 20 | P3 |
| L33 | 插件系统 | 3 | 7 | 3 | 8 | 8 | P4 |
| L34 | 自定义渲染器 | 4 | 4 | 5 | 5 | 16 | P4 |
| L35 | Webhook隧道 | 3 | 4 | 4 | 5 | 10 | P4 |
| L36 | API市场 | 2 | 4 | 3 | 8 | 3 | P4 |

---

## 五、分阶段交付计划

### Phase 0（基础设施，2周）
- G1: .helios 文件格式定义
- G2: 集合即目录（存储层重构）
- H9: 密钥保险箱（安全基线）

### Phase 1（开发者体验核心，3周）
- J20: 智能补全引擎
- J27: 请求标签与分类
- J28: 性能剖析器
- K29: CI 配置文件

### Phase 2（协作与安全，3周）
- H10: OAuth 2.0
- G3: Git 感知
- G4: .heliosignore
- I14: API 文档生成

### Phase 3（CI/CD 闭环，3周）
- K30: CI 脚手架生成
- I16: Mock 服务器
- J23: 请求模板库
- H12: 签名请求

### Phase 4（高级功能，4周）
- J21: 请求链可视化
- I15: Breaking Changes 检测
- K31: 滑动窗口回归
- G5: 请求快照与回滚

### Phase 5（生态建设，持续）
- L33: 插件系统
- L34: 自定义渲染器
- L35: Webhook 隧道
- L36: API 市场

---

## 六、与竞品的核心差异化总结

| 维度 | Postman | Bruno | HTTPie | Hurl | Helios |
|------|---------|-------|--------|------|--------|
| 交互形态 | GUI | GUI | CLI | CLI | TUI |
| Git友好 | 云同步 | 文件系统 | 无 | 纯文本 | 文件+Git感知+变更评审 |
| 安全存储 | 云加密 | .env | 无 | 无 | Keychain+Vault |
| CI/CD | Newman | CLI | 无 | --test | .helios-ci.yml+脚手架 |
| 测试编排 | Collection Runner | Runner | 无 | 文件链式 | 场景编排+依赖链+可视化 |
| Mock | 云Mock | 无 | 无 | 无 | 本地Mock+录制回放 |
| 文档 | 云文档 | 无 | 无 | 无 | 本地生成+serve |
| 插件 | 商店 | 无 | Python插件 | 无 | WASM插件 |
| 性能剖析 | 无 | 无 | -w | 无 | 时序剖析+对比 |
| 变更检测 | 无 | 无 | 无 | 无 | Breaking Changes检测 |
| 目标用户 | 所有人 | 个人开发者 | 快速请求 | 测试工程师 | 终端原生的专业API开发者 |

**Helios 的定位**：

不是"终端版 Postman"，而是"API 开发的 GitLab"——
一个以 Git 工作流为核心、终端原生的 API 全生命周期平台，
让 API 定义、测试、文档、监控都在终端里完成，
天然可协作（通过 Git）、可审计（通过快照）、可 CI/CD（通过配置文件）。

---

## 七、MVP 验证指标

Phase 0 上线后追踪：

1. GitHub Star 增速（对比同类项目首月）
2. .helios 文件格式采用率（新用户 vs 迁移用户）
3. CI 集成率（有多少用户在 CI 中使用 helios ci run）
4. 密钥保险箱使用率（安全功能的刚需验证）
5. 补全引擎使用频次（DX 改善的体感指标）
