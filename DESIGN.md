# Helios Feature Design Document

## 20 New Features — TDD Implementation Plan

Each feature follows strict RED-GREEN-REFACTOR cycle.
Test command: `cargo test`

---

### PR1: clone-request
**Goal:** Press `C` (Shift+c) in sidebar to duplicate selected request into same collection.
**Files:** shortcuts.rs, app.rs, events.rs
**Tests:**
- test_clone_request_creates_duplicate_with_new_id
- test_clone_request_appends_name_suffix
- test_clone_request_noop_when_on_collection_header
- test_clone_request_noop_when_on_environment_tab

---

### PR2: export-curl
**Goal:** `to_curl()` method on Request model that generates a valid curl command string.
**Files:** models.rs (add method), tests in models.rs
**Tests:**
- test_to_curl_get_basic
- test_to_curl_post_with_json_body
- test_to_curl_with_headers
- test_to_curl_with_auth_bearer
- test_to_curl_with_auth_basic
- test_to_curl_with_query_params

---

### PR3: import-curl
**Goal:** `from_curl(input: &str) -> Result<Request>` that parses a curl command string.
**Files:** new module curl_parser.rs, cli.rs (add import-curl subcommand)
**Tests:**
- test_parse_curl_get_basic
- test_parse_curl_post_with_data
- test_parse_curl_with_header
- test_parse_curl_with_multiple_headers
- test_parse_curl_method_flag
- test_parse_curl_invalid_input

---

### PR4: request-timeout
**Goal:** Add configurable timeout (default 30s) to HTTP requests.
**Files:** http_client.rs, models.rs (add timeout_secs field to Request), config.rs
**Tests:**
- test_default_timeout_is_30_seconds
- test_custom_timeout_is_applied
- test_timeout_rejects_zero

---

### PR5: response-raw-toggle
**Goal:** `raw_mode: bool` field on App. When true, response body shows unformatted text.
**Files:** app.rs, ui.rs
**Tests:**
- test_toggle_raw_mode_flips_state
- test_raw_mode_default_false

---

### PR6: response-search
**Goal:** `response_search_query: Option<String>` and matching logic to highlight search hits in response.
**Files:** app.rs
**Tests:**
- test_search_response_finds_match
- test_search_response_no_match
- test_search_response_case_insensitive

---

### PR7: cookie-jar
**Goal:** Use reqwest::cookie::Jar to persist cookies across requests in a session.
**Files:** http_client.rs
**Tests:**
- test_cookie_jar_stores_cookies
- test_cookie_jar_sends_cookies_on_next_request

---

### PR8: generate-code
**Goal:** `to_python(request: &str)` and `to_javascript(request: &str)` code generators.
**Files:** new module code_gen.rs
**Tests:**
- test_to_python_get_request
- test_to_python_post_json
- test_to_javascript_get_request
- test_to_javascript_post_json

---

### PR9: custom-themes
**Goal:** Load theme colors from `~/.config/helios/theme.toml` with fallback to defaults.
**Files:** config.rs, new module theme.rs
**Tests:**
- test_default_theme_values
- test_load_theme_from_file
- test_load_theme_missing_file_uses_defaults
- test_load_theme_partial_file_merges_with_defaults

---

### PR10: duplicate-collection
**Goal:** Press `D` (Shift+d) in sidebar to duplicate entire collection with new IDs.
**Files:** shortcuts.rs, app.rs, events.rs
**Tests:**
- test_duplicate_collection_creates_copy_with_new_ids
- test_duplicate_collection_appends_copy_suffix
- test_duplicate_collection_preserves_all_requests

---

### PR11: request-notes
**Goal:** `notes: String` field on Request, displayed below URL bar when non-empty.
**Files:** models.rs, app.rs, ui.rs
**Tests:**
- test_request_notes_default_empty
- test_request_notes_serialization
- test_request_notes_roundtrip_json

---

### PR12: env-variable-highlight
**Goal:** `resolve_env_vars(text: &str, env: &Environment) -> String` that replaces `{{var}}` patterns.
**Files:** utils.rs (enhance existing replace_variables)
**Tests:**
- test_resolve_single_variable
- test_resolve_multiple_variables
- test_resolve_unknown_variable_keeps_placeholder
- test_resolve_empty_env

---

### PR13: response-time-color
**Goal:** Color-code response time: green <200ms, yellow <1000ms, red >=1000ms.
**Files:** ui.rs
**Tests:**
- test_response_time_color_green
- test_response_time_color_yellow
- test_response_time_color_red

---

### PR14: status-code-description
**Goal:** Show human-readable description next to HTTP status codes (e.g., "200 OK", "404 Not Found").
**Files:** new function in utils.rs
**Tests:**
- test_status_description_200
- test_status_description_404
- test_status_description_500
- test_status_description_unknown

---

### PR15: request-size-display
**Goal:** Show total request size (headers + body) in the status bar.
**Files:** app.rs
**Tests:**
- test_request_size_empty
- test_request_size_with_body
- test_request_size_with_headers

---

### PR16: response-download
**Goal:** Press `S` (Shift+s) in response pane to save body to file.
**Files:** shortcuts.rs, app.rs, events.rs
**Tests:**
- test_save_response_creates_file
- test_save_response_empty_body_skips

---

### PR17: auto-format-body
**Goal:** Auto-format JSON body before sending if body_type is Json.
**Files:** http_client.rs
**Tests:**
- test_auto_format_valid_json
- test_auto_format_invalid_json_keeps_original
- test_auto_format_non_json_keeps_original

---

### PR18: history-filter
**Goal:** Filter history by HTTP method (GET/POST/etc) and status range.
**Files:** history.rs
**Tests:**
- test_filter_by_method_get
- test_filter_by_method_post
- test_filter_by_status_success
- test_filter_by_status_error
- test_filter_combined

---

### PR19: keyboard-help-overlay
**Goal:** Press `?` to show a full-screen help overlay with all shortcuts.
**Files:** app.rs, ui.rs
**Tests:**
- test_help_overlay_toggles
- test_help_overlay_default_hidden

---

### PR20: request-rename
**Goal:** Press `n` in sidebar to rename selected request inline.
**Files:** shortcuts.rs (already has EditRequestName), app.rs, events.rs
**Tests:**
- test_rename_request_updates_name
- test_rename_request_empty_name_rejected
- test_rename_request_persists
