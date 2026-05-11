# Helios Feature Design v2 — Apifox 功能迁移

基于 Apifox 真实功能，迁移 20 个功能到 Helios TUI 客户端。
每个功能严格 TDD（RED→GREEN→REFACTOR）。

---

## A. 请求构建增强 (4个)

### A1: 前置脚本 (pre-script)
**来源**: Apifox 前后置操作 & 脚本
**设计**:
- 新增 `src/scripting.rs` 模块
- Request 新增 `pre_script: Option<String>` 字段
- 脚本语法: `@header X-Timestamp={{$timestamp}}` 注入 header
- 脚本语法: `@set var_name={{$.json.path}}` 从上次响应提取
- 仅支持声明式脚本（不引入 JS 引擎，保持轻量）
**文件**: models.rs, scripting.rs, http_client.rs
**测试**: test_pre_script_sets_header, test_pre_script_injects_timestamp, test_pre_script_none_noop

### A2: 请求体模板 (body template)
**来源**: Apifox 请求体示例
**设计**:
- 增强现有 `utils::replace_variables()`
- 支持内置变量: `{{$timestamp}}`, `{{$uuid}}`, `{{$randomInt}}`, `{{$randomStr}}`
- Body 编辑时实时预览替换后的值（状态栏显示）
**文件**: utils.rs
**测试**: test_resolve_timestamp, test_resolve_uuid, test_resolve_random_int, test_resolve_nested

### A3: GraphQL 支持
**来源**: Apifox GraphQL 接口
**设计**:
- Request 新增 `graphql_query: Option<String>` 和 `graphql_variables: Option<String>`
- BodyType 增加 `Graphql` 变体
- 发送时自动构造 `{"query": "...", "variables": {...}}`
- URL 栏按 `g` 切换 REST/GraphQL 模式
**文件**: models.rs, http_client.rs, shortcuts.rs, events.rs
**测试**: test_graphql_body_construction, test_graphql_with_variables, test_graphql_body_type_display

### A4: multipart/form-data 文件上传
**来源**: Apifox 文件上传
**设计**:
- BodyType 增加 `FormData` 变体
- Request 新增 `form_data: Vec<FormDataItem>` 字段
- FormDataItem { key, value, is_file: bool, file_path: Option<PathBuf> }
- http_client 使用 reqwest multipart builder
**文件**: models.rs, http_client.rs
**测试**: test_form_data_item_creation, test_form_data_serialization, test_multipart_builder_text_fields

---

## B. 响应分析增强 (4个)

### B5: 响应断言 (assertions)
**来源**: Apifox 自动化测试-断言
**设计**:
- 新增 `src/assertions.rs` 模块
- Collection 新增 `assertions: Vec<Assertion>` 字段
- Assertion { path: String, operator: AssertOp, expected: String }
- AssertOp: Equals, NotEquals, Contains, GreaterThan, LessThan, Exists
- 路径支持: `status`, `body.key`, `header.Content-Type`
- 集合运行时自动执行断言
**文件**: models.rs, assertions.rs
**测试**: test_assert_status_equals, test_assert_body_contains, test_assert_header_exists, test_assert_numeric_compare

### B6: 响应提取变量
**来源**: Apifox 后置操作-提取变量
**设计**:
- Collection 新增 `extractions: Vec<Extraction>` 字段
- Extraction { var_name: String, json_path: String }
- 执行集合时，从响应 JSON 提取值存入环境变量
- 支持 `$.data.token` 形式的 JSONPath
**文件**: models.rs, extraction.rs
**测试**: test_extract_simple_path, test_extract_nested_path, test_extract_missing_path_returns_none

### B7: JSONPath 查询高亮
**来源**: Apifox 响应 JSON 树
**设计**:
- Response 面板增加 JSONPath 输入（按 `/` 激活）
- 输入 `$.data[0].name` 高亮匹配的值
- 简易 JSONPath 解析器（不引入外部依赖）
**文件**: jsonpath.rs (新模块), ui.rs
**测试**: test_jsonpath_root, test_jsonpath_nested, test_jsonpath_array_index, test_jsonpath_wildcard

### B8: 响应对比 (diff)
**来源**: Apifox 响应对比
**设计**:
- App 新增 `previous_response: Option<Response>` 字段
- 按 `D` 在 Response 面板切换 diff 模式
- diff 算法: 逐行对比，新增行绿色，删除行红色
- 使用简单的 LCS (最长公共子序列) 算法
**文件**: app.rs, diff.rs (新模块), ui.rs
**测试**: test_diff_identical, test_diff_added_lines, test_diff_removed_lines, test_lcs_basic

---

## C. 环境 & 变量管理 (3个)

### C9: 全局变量
**来源**: Apifox 全局变量
**设计**:
- AppData 新增 `global_variables: HashMap<String, String>`
- 环境变量优先级: 环境变量 > 全局变量
- TUI 新增 Global tab 在 Environments 旁边
- CLI: `helios global set key value`
**文件**: models.rs, storage.rs, app.rs, cli.rs
**测试**: test_global_variable_default_empty, test_global_variable_override, test_global_variable_serialization

### C10: 内置动态变量
**来源**: Apifox 内置变量
**设计**:
- 增强 `utils::replace_variables()`
- 支持: `{{$timestamp}}` (毫秒时间戳), `{{$uuid}}` (UUID v4), `{{$randomInt}}` (0-9999), `{{$randomStr}}` (8位随机字符串), `{{$date}}` (YYYY-MM-DD)
**文件**: utils.rs
**测试**: test_builtin_timestamp, test_builtin_uuid, test_builtin_random_int, test_builtin_date

### C11: 变量自动替换
**来源**: Apifox 环境变量替换
**设计**:
- 在 `send_request` 前，对 URL, Headers, Body 执行 `replace_variables()`
- 已有 `replace_variables` 函数，需在 http_client 中调用
- 未解析的变量保留原样（不报错）
**文件**: http_client.rs
**测试**: test_url_variable_replacement, test_header_variable_replacement, test_body_variable_replacement

---

## D. 测试流程 (3个)

### D12: 测试场景编排
**来源**: Apifox 自动化测试-场景
**设计**:
- Collection 新增 `scenario: Vec<ScenarioStep>` 字段
- ScenarioStep { request_index: usize, delay_ms: u64, skip_on_fail: bool }
- `helios run "Collection" --scenario` 按场景顺序执行
- 变量在请求间自动传递
**文件**: models.rs, scenario.rs
**测试**: test_scenario_step_creation, test_scenario_variable_passing, test_scenario_skip_on_fail

### D13: 测试报告
**来源**: Apifox 测试报告
**设计**:
- 集合运行后生成结构化报告
- 包含: 总请求数、通过数、失败数、总耗时、每个请求的详情
- 输出格式: 终端彩色表格 + 可选 JSON 文件导出
- `helios run "Collection" --report report.json`
**文件**: report.rs (新模块)
**测试**: test_report_generation, test_report_pass_rate, test_report_json_export

### D14: 请求依赖链
**来源**: Apifox 前置操作-依赖请求
**设计**:
- ScenarioStep 支持 `depends_on: Option<usize>` 字段
- 依赖请求的响应自动注入到当前请求的变量中
- 如: 登录请求的 token 自动设为 `{{auth_token}}`
**文件**: scenario.rs, models.rs
**测试**: test_dependency_chain_basic, test_dependency_variable_injection

---

## E. 交互体验 (4个)

### E15: 请求搜索
**来源**: Apifox 全局搜索
**设计**:
- 按 `/` 打开搜索弹窗（已有 HistorySearch dialog 类型可复用）
- 搜索范围: 所有集合中的请求名 + URL
- 实时过滤，Enter 加载选中请求
**文件**: app.rs, events.rs, shortcuts.rs
**测试**: test_search_by_name, test_search_by_url, test_search_case_insensitive

### E16: 快捷导入 curl
**来源**: Apifox 导入
**设计**:
- 已有 `curl_parser::parse_curl()` (PR3)
- TUI 中按 `I` 打开 curl 导入弹窗
- 粘贴 curl 命令，自动解析并加载到当前请求
**文件**: events.rs, shortcuts.rs, app.rs
**测试**: test_import_curl_loads_request, test_import_curl_invalid_shows_error

### E17: 请求标签页
**来源**: Apifox 多 Tab
**设计**:
- App 新增 `open_tabs: Vec<TabInfo>` 和 `active_tab: usize`
- TabInfo { request: Request, source: Option<(usize, usize)> }
- `Ctrl+T` 新建标签，`Ctrl+W` 关闭标签，`Ctrl+1..9` 切换
- 顶栏显示标签名
**文件**: app.rs, ui.rs, shortcuts.rs, events.rs
**测试**: test_open_tab, test_close_tab, test_switch_tab, test_tab_limit

### E18: 历史对比
**来源**: Apifox 历史记录
**设计**:
- 历史弹窗中按 `c` 进入对比模式
- 选择两条历史记录，显示请求差异（URL、Headers、Body）
- 复用 B8 的 diff 算法
**文件**: app.rs, diff.rs
**测试**: test_history_diff_requests, test_history_diff_responses

---

## F. 数据格式 (2个)

### F19: Swagger/OpenAPI 导入
**来源**: Apifox 导入 Swagger
**设计**:
- 新增 `src/openapi.rs` 模块
- 支持 OpenAPI 3.0 JSON 格式
- 解析 paths → 转换为 Collection + Requests
- `helios import api.json --format openapi`
- TUI: 导入弹窗支持 `openapi` 格式
**文件**: openapi.rs (新模块), cli.rs
**测试**: test_parse_openapi_basic, test_parse_openapi_with_params, test_parse_openapi_with_body

### F20: HAR 导入
**来源**: Apifox 导入 HAR
**设计**:
- 新增 `src/har.rs` 模块
- 解析 HAR JSON 的 `log.entries` → 转换为 Requests
- `helios import network.har --format har`
- 保留请求方法、URL、Headers、Body
**文件**: har.rs (新模块), cli.rs
**测试**: test_parse_har_basic, test_parse_har_with_post_body, test_parse_har_with_headers

---

## 实现顺序（按依赖关系）

```
Phase 1 (基础): C10, C11, A2  → 动态变量 + 自动替换 + 模板
Phase 2 (分析): B5, B6, B7    → 断言 + 提取 + JSONPath
Phase 3 (构建): A1, A3, A4    → 前置脚本 + GraphQL + 文件上传
Phase 4 (流程): D12, D13, D14 → 场景编排 + 报告 + 依赖链
Phase 5 (体验): E15, E16, E17, E18 → 搜索 + curl导入 + 标签页 + 历史对比
Phase 6 (格式): F19, F20      → OpenAPI + HAR 导入
```

总计: 20 个功能，预计 60+ 个 TDD 测试用例
