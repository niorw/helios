use crate::models::{
    AppData, Auth, BodyType, Collection, HistoryItem, HttpMethod, KeyValue, Request,
    Response,
};
use crate::storage::Storage;
use crate::history::{HistoryManager, HistoryStorage};
use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActivePane {
    Sidebar,
    UrlBar,
    RequestTabs,
    Response,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RequestTab {
    Params,
    Headers,
    Body,
    Auth,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResponseTab {
    Body,
    Headers,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SidebarTab {
    Collections,
    Environments,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditingField {
    Url,
    RequestName,
    Body,
    ParamKey(usize),
    ParamValue(usize),
    HeaderKey(usize),
    HeaderValue(usize),
    AuthToken,
    AuthUsername,
    AuthPassword,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DialogType {
    None,
    ExportCollection,
    NewCollection,
    DeleteConfirm,
    RequestName,
    History,           // 历史记录弹窗
    HistorySearch,     // 历史搜索弹窗
}

#[derive(Debug, Clone)]
pub enum DeleteTarget {
    Collection(usize),
    Request(usize, usize), // collection_index, request_index
}

pub struct App {
    pub running: bool,
    pub input_mode: InputMode,
    pub active_pane: ActivePane,
    pub sidebar_tab: SidebarTab,
    pub request_tab: RequestTab,
    pub response_tab: ResponseTab,
    pub data: AppData,
    pub storage: Storage,

    pub current_request: Request,
    pub response: Option<Response>,
    pub editing_field: Option<EditingField>,
    pub edit_buffer: String,
    pub cursor_pos: usize,

    pub sidebar_selected: usize,
    pub collection_expanded: Vec<String>,

    pub param_list_state: ratatui::widgets::TableState,
    pub header_list_state: ratatui::widgets::TableState,

    pub status_message: Option<(String, u64)>, // (message, expiry_tick)
    pub tick: u64,
    pub loading: bool,
    pub response_scroll: (u16, u16),
    pub pending_send: bool,

    pub dialog_type: DialogType,
    pub dialog_buffer: String,
    pub dialog_cursor: usize,
    pub dialog_message: String,
    pub dialog_option_selected: bool, // For choice dialogs: true = confirm, false = cancel

    pub pending_window_prefix: bool,
    pub pending_window_expiry: u64,

    pub current_request_source: Option<(usize, usize)>, // (collection_index, request_index)
    pub pending_delete: Option<DeleteTarget>,
    
    // History feature
    pub history_manager: HistoryManager,
    pub history_storage: HistoryStorage,
    pub history_selected: usize,
}

impl App {
    pub fn new() -> Result<Self> {
        let storage = Storage::new()?;
        let data = storage.load()?;
        
        // Initialize history storage
        let proj_dirs = directories::ProjectDirs::from("com", "helios", "helios")
            .ok_or_else(|| anyhow::anyhow!("Could not determine project directories"))?;
        let data_dir = proj_dirs.data_dir().to_path_buf();
        let history_storage = HistoryStorage::new(data_dir.clone());
        let history_manager = history_storage.load().unwrap_or_default();

        let mut app = Self {
            running: true,
            input_mode: InputMode::Normal,
            active_pane: ActivePane::Sidebar,
            sidebar_tab: SidebarTab::Collections,
            request_tab: RequestTab::Headers,
            response_tab: ResponseTab::Body,
            data,
            storage,
            current_request: Request::default(),
            response: None,
            editing_field: None,
            edit_buffer: String::new(),
            cursor_pos: 0,
            sidebar_selected: 0,
            collection_expanded: Vec::new(),
            param_list_state: ratatui::widgets::TableState::default(),
            header_list_state: ratatui::widgets::TableState::default(),
            status_message: None,
            tick: 0,
            loading: false,
            response_scroll: (0, 0),
            pending_send: false,
            dialog_type: DialogType::None,
            dialog_buffer: String::new(),
            dialog_cursor: 0,
            dialog_message: String::new(),
            dialog_option_selected: false,
            pending_window_prefix: false,
            pending_window_expiry: 0,
            current_request_source: None,
            pending_delete: None,
            history_manager,
            history_storage,
            history_selected: 0,
        };

        app.current_request.url = crate::config::DEFAULT_URL.to_string();
        app.current_request.headers.push(KeyValue {
            key: "Accept".to_string(),
            value: "application/json".to_string(),
            enabled: true,
        });
        app.param_list_state.select(Some(0));
        app.header_list_state.select(Some(0));

        // Seed demo data if empty
        if app.data.collections.is_empty() {
            let mk = |name: &str, method: HttpMethod, url: &str, headers: Vec<(&str, &str)>, body: &str, body_type: BodyType| -> Request {
                Request {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: name.to_string(),
                    method,
                    url: url.to_string(),
                    headers: headers.into_iter().map(|(k, v)| KeyValue { key: k.to_string(), value: v.to_string(), enabled: true }).collect(),
                    params: vec![],
                    body: body.to_string(),
                    body_type,
                    auth: Auth::None,..Default::default()
                }
            };

            app.data.collections = vec![
                Collection {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: "🌐 httpbin.org".to_string(),
                    requests: vec![
                        mk("GET 查询参数", HttpMethod::GET, "https://httpbin.org/get?foo=bar&baz=qux", vec![("Accept","application/json")], "", BodyType::None),
                        mk("POST JSON", HttpMethod::POST, "https://httpbin.org/post", vec![("Content-Type","application/json"),("Accept","application/json")], r#"{"username":"helios","password":"demo123"}"#, BodyType::Json),
                        mk("PUT 更新", HttpMethod::PUT, "https://httpbin.org/put", vec![("Content-Type","application/json"),("Accept","application/json")], r#"{"id":1,"name":"updated"}"#, BodyType::Json),
                        mk("PATCH 部分更新", HttpMethod::PATCH, "https://httpbin.org/patch", vec![("Content-Type","application/json"),("Accept","application/json")], r#"{"name":"patched"}"#, BodyType::Json),
                        mk("DELETE 资源", HttpMethod::DELETE, "https://httpbin.org/delete", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET IP 地址", HttpMethod::GET, "https://httpbin.org/ip", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET User-Agent", HttpMethod::GET, "https://httpbin.org/user-agent", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET Base64 解码", HttpMethod::GET, "https://httpbin.org/base64/SGVsbG8gV29ybGQ=", vec![("Accept","text/plain")], "", BodyType::None),
                    ],
                    created_at: chrono::Local::now(),
                },
                Collection {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: "📝 JSONPlaceholder".to_string(),
                    requests: vec![
                        mk("GET 文章列表", HttpMethod::GET, "https://jsonplaceholder.typicode.com/posts", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 单篇文章", HttpMethod::GET, "https://jsonplaceholder.typicode.com/posts/1", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 文章评论", HttpMethod::GET, "https://jsonplaceholder.typicode.com/posts/1/comments", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 用户列表", HttpMethod::GET, "https://jsonplaceholder.typicode.com/users", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 待办事项", HttpMethod::GET, "https://jsonplaceholder.typicode.com/todos/1", vec![("Accept","application/json")], "", BodyType::None),
                        mk("POST 新建文章", HttpMethod::POST, "https://jsonplaceholder.typicode.com/posts", vec![("Content-Type","application/json"),("Accept","application/json")], r#"{"title":"Helios Demo","body":"A blazing-fast terminal API client.","userId":1}"#, BodyType::Json),
                        mk("PUT 更新文章", HttpMethod::PUT, "https://jsonplaceholder.typicode.com/posts/1", vec![("Content-Type","application/json"),("Accept","application/json")], r#"{"id":1,"title":"Updated","body":"Updated body","userId":1}"#, BodyType::Json),
                        mk("DELETE 文章", HttpMethod::DELETE, "https://jsonplaceholder.typicode.com/posts/1", vec![("Accept","application/json")], "", BodyType::None),
                    ],
                    created_at: chrono::Local::now(),
                },
                Collection {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: "👤 ReqRes".to_string(),
                    requests: vec![
                        mk("GET 用户列表", HttpMethod::GET, "https://reqres.in/api/users?page=2", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 单用户", HttpMethod::GET, "https://reqres.in/api/users/2", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 资源列表", HttpMethod::GET, "https://reqres.in/api/unknown", vec![("Accept","application/json")], "", BodyType::None),
                        mk("POST 登录", HttpMethod::POST, "https://reqres.in/api/login", vec![("Content-Type","application/json"),("Accept","application/json")], r#"{"email":"eve.holt@reqres.in","password":"cityslicka"}"#, BodyType::Json),
                        mk("POST 注册", HttpMethod::POST, "https://reqres.in/api/register", vec![("Content-Type","application/json"),("Accept","application/json")], r#"{"email":"eve.holt@reqres.in","password":"pistol"}"#, BodyType::Json),
                        mk("PUT 更新用户", HttpMethod::PUT, "https://reqres.in/api/users/2", vec![("Content-Type","application/json"),("Accept","application/json")], r#"{"name":"Helios","job":"API Client"}"#, BodyType::Json),
                        mk("DELETE 用户", HttpMethod::DELETE, "https://reqres.in/api/users/2", vec![("Accept","application/json")], "", BodyType::None),
                    ],
                    created_at: chrono::Local::now(),
                },
                Collection {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: "🌤️ 天气与地理".to_string(),
                    requests: vec![
                        mk("GET 北京实时天气", HttpMethod::GET, "https://api.open-meteo.com/v1/forecast?latitude=39.9&longitude=116.4&current=temperature_2m,relative_humidity_2m,weather_code", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 上海实时天气", HttpMethod::GET, "https://api.open-meteo.com/v1/forecast?latitude=31.2&longitude=121.5&current=temperature_2m,relative_humidity_2m,weather_code", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 深圳实时天气", HttpMethod::GET, "https://api.open-meteo.com/v1/forecast?latitude=22.5&longitude=114.1&current=temperature_2m,relative_humidity_2m,weather_code", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 上海天气文本", HttpMethod::GET, "https://wttr.in/Shanghai?format=j1", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET IP 地理信息", HttpMethod::GET, "https://ipapi.co/json/", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 上海时间", HttpMethod::GET, "http://worldtimeapi.org/api/timezone/Asia/Shanghai", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 纽约时间", HttpMethod::GET, "http://worldtimeapi.org/api/timezone/America/New_York", vec![("Accept","application/json")], "", BodyType::None),
                    ],
                    created_at: chrono::Local::now(),
                },
                Collection {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: "💰 金融数据".to_string(),
                    requests: vec![
                        mk("GET 美元汇率", HttpMethod::GET, "https://api.exchangerate-api.com/v4/latest/USD", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 人民币汇率", HttpMethod::GET, "https://api.exchangerate-api.com/v4/latest/CNY", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 欧元汇率", HttpMethod::GET, "https://api.exchangerate-api.com/v4/latest/EUR", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 加密货币价格", HttpMethod::GET, "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin,ethereum&vs_currencies=usd,cny", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 比特币历史", HttpMethod::GET, "https://api.coingecko.com/api/v3/coins/bitcoin/market_chart?vs_currency=usd&days=7", vec![("Accept","application/json")], "", BodyType::None),
                    ],
                    created_at: chrono::Local::now(),
                },
                Collection {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: "🎮 娱乐与生活".to_string(),
                    requests: vec![
                        mk("GET 宝可梦-皮卡丘", HttpMethod::GET, "https://pokeapi.co/api/v2/pokemon/pikachu", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 宝可梦-喷火龙", HttpMethod::GET, "https://pokeapi.co/api/v2/pokemon/charizard", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 随机狗狗图片", HttpMethod::GET, "https://dog.ceo/api/breeds/image/random", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 随机猫咪图片", HttpMethod::GET, "https://api.thecatapi.com/v1/images/search", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 随机用户信息", HttpMethod::GET, "https://randomuser.me/api/", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 随机笑话", HttpMethod::GET, "https://official-joke-api.appspot.com/random_joke", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 随机活动", HttpMethod::GET, "https://www.boredapi.com/api/activity", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 无聊事实", HttpMethod::GET, "https://uselessfacts.jsph.pl/api/v2/facts/random", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 吉卜力电影", HttpMethod::GET, "https://ghibliapi.vercel.app/films", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 一言", HttpMethod::GET, "https://v1.hitokoto.cn/", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 每日诗词", HttpMethod::GET, "https://v1.jinrishici.com/all.json", vec![("Accept","application/json")], "", BodyType::None),
                    ],
                    created_at: chrono::Local::now(),
                },
                Collection {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: "📻 广播与资讯".to_string(),
                    requests: vec![
                        mk("GET HackerNews 热榜", HttpMethod::GET, "https://hacker-news.firebaseio.com/v0/topstories.json", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET HackerNews 单条", HttpMethod::GET, "https://hacker-news.firebaseio.com/v0/item/8863.json", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET Reddit 编程", HttpMethod::GET, "https://www.reddit.com/r/programming.json", vec![("Accept","application/json"),("User-Agent", crate::config::DEFAULT_USER_AGENT)], "", BodyType::None),
                        mk("GET 中国电台", HttpMethod::GET, "http://de1.api.radio-browser.info/json/stations/search?limit=10&countrycode=CN", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 全球热门电台", HttpMethod::GET, "http://de1.api.radio-browser.info/json/stations/topvote/10", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET 知乎日报", HttpMethod::GET, "https://news-at.zhihu.com/api/4/news/latest", vec![("Accept","application/json")], "", BodyType::None),
                    ],
                    created_at: chrono::Local::now(),
                },
                Collection {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: "📡 技术与开发".to_string(),
                    requests: vec![
                        mk("GET GitHub Torvalds", HttpMethod::GET, "https://api.github.com/users/torvalds", vec![("Accept","application/json"),("User-Agent", crate::config::DEFAULT_USER_AGENT)], "", BodyType::None),
                        mk("GET GitHub Rust 仓库", HttpMethod::GET, "https://api.github.com/repos/rust-lang/rust", vec![("Accept","application/json"),("User-Agent", crate::config::DEFAULT_USER_AGENT)], "", BodyType::None),
                        mk("GET GitHub Trending", HttpMethod::GET, "https://api.github.com/search/repositories?q=language:rust&sort=stars&order=desc&per_page=5", vec![("Accept","application/json"),("User-Agent", crate::config::DEFAULT_USER_AGENT)], "", BodyType::None),
                        mk("GET GitHub Events", HttpMethod::GET, "https://api.github.com/events?per_page=5", vec![("Accept","application/json"),("User-Agent", crate::config::DEFAULT_USER_AGENT)], "", BodyType::None),
                        mk("GET NPM 包信息", HttpMethod::GET, "https://registry.npmjs.org/react/latest", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET Docker Hub 镜像", HttpMethod::GET, "https://hub.docker.com/v2/repositories/library/rust/", vec![("Accept","application/json")], "", BodyType::None),
                    ],
                    created_at: chrono::Local::now(),
                },
            ];
            let _ = app.save();
        }

        Ok(app)
    }

    pub fn save(&self) -> Result<()> {
        self.storage.save(&self.data)?;
        Ok(())
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), self.tick + crate::config::STATUS_MESSAGE_TIMEOUT_TICKS));
    }

    pub fn tick(&mut self) {
        self.tick += 1;
        if let Some((_, expiry)) = self.status_message {
            if self.tick >= expiry {
                self.status_message = None;
            }
        }
        if self.pending_window_prefix && self.tick >= self.pending_window_expiry {
            self.pending_window_prefix = false;
        }
    }

    pub fn move_focus_next(&mut self) {
        self.active_pane = match self.active_pane {
            ActivePane::Sidebar => ActivePane::UrlBar,
            ActivePane::UrlBar => ActivePane::RequestTabs,
            ActivePane::RequestTabs => ActivePane::Response,
            ActivePane::Response => ActivePane::Sidebar,
        };
    }

    pub fn move_focus_prev(&mut self) {
        self.active_pane = match self.active_pane {
            ActivePane::Sidebar => ActivePane::Response,
            ActivePane::UrlBar => ActivePane::Sidebar,
            ActivePane::RequestTabs => ActivePane::UrlBar,
            ActivePane::Response => ActivePane::RequestTabs,
        };
    }

    pub fn sidebar_item_count(&self) -> usize {
        match self.sidebar_tab {
            SidebarTab::Collections => {
                let mut count = self.data.collections.len();
                for c in &self.data.collections {
                    if self.collection_expanded.contains(&c.id) {
                        count += c.requests.len();
                    }
                }
                count
            }
            SidebarTab::Environments => self.data.environments.len(),
        }
    }

    pub fn next_sidebar_item(&mut self) {
        let max = self.sidebar_item_count();
        if max > 0 {
            self.sidebar_selected = (self.sidebar_selected + 1) % max;
        }
    }

    pub fn prev_sidebar_item(&mut self) {
        let max = self.sidebar_item_count();
        if max > 0 {
            self.sidebar_selected = (self.sidebar_selected + max - 1) % max;
        }
    }

    pub fn try_load_selected_collection_request(&mut self) -> bool {
        if self.sidebar_tab != SidebarTab::Collections {
            return false;
        }
        let mut idx = 0;
        for (ci, col) in self.data.collections.iter().enumerate() {
            if self.sidebar_selected == idx {
                return false; // selected a collection header, not a request
            }
            idx += 1;
            if self.collection_expanded.contains(&col.id) {
                for (ri, req) in col.requests.iter().enumerate() {
                    if self.sidebar_selected == idx {
                        let name = req.name.clone();
                        self.load_request(req.clone(), Some((ci, ri)));
                        self.set_status(format!("Loaded: {}", name));
                        return true;
                    }
                    idx += 1;
                }
            }
        }
        false
    }

    pub fn get_selected_collection_index(&self) -> Option<usize> {
        if self.sidebar_tab != SidebarTab::Collections {
            return None;
        }
        let mut idx = 0;
        for (ci, col) in self.data.collections.iter().enumerate() {
            if self.sidebar_selected == idx {
                return Some(ci);
            }
            idx += 1;
            if self.collection_expanded.contains(&col.id) {
                idx += col.requests.len();
            }
        }
        None
    }

    /// Returns the collection index that contains the currently selected sidebar item.
    /// Works whether the selection is on a collection header or one of its requests.
    pub fn get_selected_or_parent_collection_index(&self) -> Option<usize> {
        if self.sidebar_tab != SidebarTab::Collections {
            return None;
        }
        let mut idx = 0;
        for (ci, col) in self.data.collections.iter().enumerate() {
            if self.sidebar_selected == idx {
                return Some(ci);
            }
            idx += 1;
            if self.collection_expanded.contains(&col.id) {
                for _ in &col.requests {
                    if self.sidebar_selected == idx {
                        return Some(ci);
                    }
                    idx += 1;
                }
            }
        }
        None
    }

    pub fn add_param(&mut self) {
        self.current_request.params.push(KeyValue::default());
        self.param_list_state
            .select(Some(self.current_request.params.len().saturating_sub(1)));
    }

    pub fn add_header(&mut self) {
        self.current_request.headers.push(KeyValue::default());
        self.header_list_state
            .select(Some(self.current_request.headers.len().saturating_sub(1)));
    }

    pub fn remove_param(&mut self, idx: usize) {
        if idx < self.current_request.params.len() {
            self.current_request.params.remove(idx);
            let max = self.current_request.params.len().saturating_sub(1);
            let sel = self
                .param_list_state
                .selected()
                .unwrap_or(0)
                .min(max);
            self.param_list_state.select(Some(sel));
        }
    }

    pub fn remove_header(&mut self, idx: usize) {
        if idx < self.current_request.headers.len() {
            self.current_request.headers.remove(idx);
            let max = self.current_request.headers.len().saturating_sub(1);
            let sel = self
                .header_list_state
                .selected()
                .unwrap_or(0)
                .min(max);
            self.header_list_state.select(Some(sel));
        }
    }

    pub fn toggle_param(&mut self, idx: usize) {
        if let Some(p) = self.current_request.params.get_mut(idx) {
            p.enabled = !p.enabled;
        }
    }

    pub fn toggle_header(&mut self, idx: usize) {
        if let Some(h) = self.current_request.headers.get_mut(idx) {
            h.enabled = !h.enabled;
        }
    }

    pub fn start_editing(&mut self, field: EditingField) {
        self.input_mode = InputMode::Editing;
        self.editing_field = Some(field);
        self.edit_buffer = match field {
            EditingField::Url => self.current_request.url.clone(),
            EditingField::RequestName => self.current_request.name.clone(),
            EditingField::Body => self.current_request.body.clone(),
            EditingField::ParamKey(i) => self
                .current_request
                .params
                .get(i)
                .map(|p| p.key.clone())
                .unwrap_or_default(),
            EditingField::ParamValue(i) => self
                .current_request
                .params
                .get(i)
                .map(|p| p.value.clone())
                .unwrap_or_default(),
            EditingField::HeaderKey(i) => self
                .current_request
                .headers
                .get(i)
                .map(|h| h.key.clone())
                .unwrap_or_default(),
            EditingField::HeaderValue(i) => self
                .current_request
                .headers
                .get(i)
                .map(|h| h.value.clone())
                .unwrap_or_default(),
            EditingField::AuthToken => match &self.current_request.auth {
                Auth::Bearer { token } => token.clone(),
                _ => String::new(),
            },
            EditingField::AuthUsername => match &self.current_request.auth {
                Auth::Basic { username, .. } => username.clone(),
                _ => String::new(),
            },
            EditingField::AuthPassword => match &self.current_request.auth {
                Auth::Basic { password, .. } => password.clone(),
                _ => String::new(),
            },
        };
        self.cursor_pos = self.edit_buffer.len();
    }

    pub fn confirm_edit(&mut self) {
        if let Some(field) = self.editing_field {
            match field {
                EditingField::Url => self.current_request.url = self.edit_buffer.clone(),
                EditingField::RequestName => self.current_request.name = self.edit_buffer.clone(),
                EditingField::Body => self.current_request.body = self.edit_buffer.clone(),
                EditingField::ParamKey(i) => {
                    if let Some(p) = self.current_request.params.get_mut(i) {
                        p.key = self.edit_buffer.clone();
                    }
                }
                EditingField::ParamValue(i) => {
                    if let Some(p) = self.current_request.params.get_mut(i) {
                        p.value = self.edit_buffer.clone();
                    }
                }
                EditingField::HeaderKey(i) => {
                    if let Some(h) = self.current_request.headers.get_mut(i) {
                        h.key = self.edit_buffer.clone();
                    }
                }
                EditingField::HeaderValue(i) => {
                    if let Some(h) = self.current_request.headers.get_mut(i) {
                        h.value = self.edit_buffer.clone();
                    }
                }
                EditingField::AuthToken => {
                    self.current_request.auth =
                        Auth::Bearer { token: self.edit_buffer.clone() };
                }
                EditingField::AuthUsername => {
                    let password = match &self.current_request.auth {
                        Auth::Basic { password, .. } => password.clone(),
                        _ => String::new(),
                    };
                    self.current_request.auth = Auth::Basic {
                        username: self.edit_buffer.clone(),
                        password,
                    };
                }
                EditingField::AuthPassword => {
                    let username = match &self.current_request.auth {
                        Auth::Basic { username, .. } => username.clone(),
                        _ => String::new(),
                    };
                    self.current_request.auth = Auth::Basic {
                        username,
                        password: self.edit_buffer.clone(),
                    };
                }
            }
        }
        // Auto-save: sync current request back to its source collection if tracked
        if let Some((ci, ri)) = self.current_request_source {
            if ci < self.data.collections.len() && ri < self.data.collections[ci].requests.len() {
                let req = self.current_request.clone();
                self.data.collections[ci].requests[ri] = req;
                let _ = self.save();
            }
        }
        self.cancel_edit();
    }

    pub fn cancel_edit(&mut self) {
        self.input_mode = InputMode::Normal;
        self.editing_field = None;
        self.edit_buffer.clear();
        self.cursor_pos = 0;
    }

    pub fn insert_char(&mut self, c: char) {
        if self.cursor_pos > self.edit_buffer.len() {
            self.cursor_pos = self.edit_buffer.len();
        }
        self.edit_buffer.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    pub fn delete_char(&mut self) {
        if self.cursor_pos > 0 && self.cursor_pos <= self.edit_buffer.len() {
            let prev = self.prev_byte_boundary(&self.edit_buffer, self.cursor_pos);
            self.edit_buffer.replace_range(prev..self.cursor_pos, "");
            self.cursor_pos = prev;
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos = self.prev_byte_boundary(&self.edit_buffer, self.cursor_pos);
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.edit_buffer.len() {
            if let Some(c) = self.edit_buffer[self.cursor_pos..].chars().next() {
                self.cursor_pos += c.len_utf8();
            }
        }
    }

    fn prev_byte_boundary(&self, s: &str, pos: usize) -> usize {
        s[..pos].char_indices().last().map(|(idx, _)| idx).unwrap_or(0)
    }

    // Dialog text editing (same Unicode-safe logic)
    pub fn dialog_insert_char(&mut self, c: char) {
        if self.dialog_cursor > self.dialog_buffer.len() {
            self.dialog_cursor = self.dialog_buffer.len();
        }
        self.dialog_buffer.insert(self.dialog_cursor, c);
        self.dialog_cursor += c.len_utf8();
    }

    pub fn dialog_delete_char(&mut self) {
        if self.dialog_cursor > 0 && self.dialog_cursor <= self.dialog_buffer.len() {
            let prev = self.prev_byte_boundary(&self.dialog_buffer, self.dialog_cursor);
            self.dialog_buffer.replace_range(prev..self.dialog_cursor, "");
            self.dialog_cursor = prev;
        }
    }

    pub fn dialog_move_cursor_left(&mut self) {
        if self.dialog_cursor > 0 {
            self.dialog_cursor = self.prev_byte_boundary(&self.dialog_buffer, self.dialog_cursor);
        }
    }

    pub fn dialog_move_cursor_right(&mut self) {
        if self.dialog_cursor < self.dialog_buffer.len() {
            if let Some(c) = self.dialog_buffer[self.dialog_cursor..].chars().next() {
                self.dialog_cursor += c.len_utf8();
            }
        }
    }

    pub fn cycle_method(&mut self) {
        self.current_request.method = match self.current_request.method {
            HttpMethod::GET => HttpMethod::POST,
            HttpMethod::POST => HttpMethod::PUT,
            HttpMethod::PUT => HttpMethod::DELETE,
            HttpMethod::DELETE => HttpMethod::PATCH,
            HttpMethod::PATCH => HttpMethod::HEAD,
            HttpMethod::HEAD => HttpMethod::OPTIONS,
            HttpMethod::OPTIONS => HttpMethod::GET,
        };
    }

    pub fn cycle_method_prev(&mut self) {
        self.current_request.method = match self.current_request.method {
            HttpMethod::GET => HttpMethod::OPTIONS,
            HttpMethod::POST => HttpMethod::GET,
            HttpMethod::PUT => HttpMethod::POST,
            HttpMethod::DELETE => HttpMethod::PUT,
            HttpMethod::PATCH => HttpMethod::DELETE,
            HttpMethod::HEAD => HttpMethod::PATCH,
            HttpMethod::OPTIONS => HttpMethod::HEAD,
        };
    }

    pub fn cycle_response_tab(&mut self) {
        self.response_tab = match self.response_tab {
            ResponseTab::Body => ResponseTab::Headers,
            ResponseTab::Headers => ResponseTab::Body,
        };
        self.response_scroll = (0, 0);
    }

    pub fn cycle_body_type(&mut self) {
        self.current_request.body_type = match self.current_request.body_type {
            BodyType::None => BodyType::Json,
            BodyType::Json => BodyType::Form,
            BodyType::Form => BodyType::Text,
            BodyType::Text => BodyType::Xml,
            BodyType::Xml => BodyType::Graphql,
            BodyType::Graphql => BodyType::FormData,
            BodyType::FormData => BodyType::None,
        };
    }

    pub fn cycle_header_key(&mut self, idx: usize) {
        if let Some(h) = self.current_request.headers.get_mut(idx) {
            let common = vec![
                "Content-Type", "Accept", "Authorization", "User-Agent",
                "Cache-Control", "X-Request-Id", "X-Api-Key", "Referer", "Origin",
            ];
            let pos = common.iter().position(|&k| k == h.key).unwrap_or(common.len() - 1);
            let next = common[(pos + 1) % common.len()];
            h.key = next.to_string();
            h.value = match next {
                "Content-Type" => "application/json".to_string(),
                "Accept" => "application/json".to_string(),
                "Authorization" => "Bearer ".to_string(),
                "User-Agent" => crate::config::DEFAULT_USER_AGENT.to_string(),
                "Cache-Control" => "no-cache".to_string(),
                _ => String::new(),
            };
        }
    }

    pub fn cycle_header_value(&mut self, idx: usize) {
        if let Some(h) = self.current_request.headers.get_mut(idx) {
            let presets: Vec<&str> = match h.key.as_str() {
                "Content-Type" => vec!["application/json", "application/x-www-form-urlencoded", "text/plain", "application/xml", "multipart/form-data"],
                "Accept" => vec!["application/json", "text/html", "application/xml", "*/*"],
                "Authorization" => vec!["Bearer ", "Basic "],
                "Cache-Control" => vec!["no-cache", "no-store", "max-age=0", "must-revalidate"],
                "User-Agent" => vec![crate::config::DEFAULT_USER_AGENT, "Mozilla/5.0", "curl/7.64.1"],
                _ => vec![""],
            };
            if !presets.is_empty() {
                let pos = presets.iter().position(|&v| h.value == v).unwrap_or(presets.len() - 1);
                let next = presets[(pos + 1) % presets.len()];
                h.value = next.to_string();
            }
        }
    }

    pub fn cycle_auth_type(&mut self) {
        self.current_request.auth = match self.current_request.auth {
            Auth::None => Auth::Bearer { token: String::new() },
            Auth::Bearer { .. } => Auth::Basic { username: String::new(), password: String::new() },
            Auth::Basic { .. } => Auth::None,
        };
    }

    pub fn new_request(&mut self) {
        self.current_request = Request::default();
        self.current_request.url = String::new();
        self.current_request_source = None;
        self.response = None;
        self.request_tab = RequestTab::Headers;
        self.param_list_state.select(Some(0));
        self.header_list_state.select(Some(0));
        self.active_pane = ActivePane::UrlBar;
        self.set_status("New request created — edit URL and press Enter to send");
    }

    pub fn activate_environment(&mut self, idx: usize) {
        if let Some(env) = self.data.environments.get(idx) {
            let id = env.id.clone();
            let name = env.name.clone();
            self.data.active_env_id = Some(id);
            let _ = self.save();
            self.set_status(format!("Activated environment '{}'", name));
        }
    }

    pub fn load_request(&mut self, req: Request, source: Option<(usize, usize)>) {
        self.current_request = req;
        self.current_request_source = source;
        self.response = None;
        self.response_scroll = (0, 0);
        self.request_tab = RequestTab::Headers;
        self.param_list_state.select(Some(0));
        self.header_list_state.select(Some(0));
    }

    pub fn add_to_history(&mut self, request: Request, response: Response) {
        let item = HistoryItem {
            id: uuid::Uuid::new_v4().to_string(),
            request,
            response,
            timestamp: chrono::Local::now(),
        };
        self.data.history.insert(0, item);
        if self.data.history.len() > 100 {
            self.data.history.truncate(100);
        }
        let _ = self.save();
    }

    pub fn get_active_env_vars(&self) -> std::collections::HashMap<String, String> {
        if let Some(id) = &self.data.active_env_id {
            if let Some(env) = self.data.environments.iter().find(|e| &e.id == id) {
                return env.variables.clone();
            }
        }
        std::collections::HashMap::new()
    }

    pub fn build_request_with_env(&self, req: &Request) -> Request {
        let vars = self.get_active_env_vars();
        let mut r = req.clone();
        r.url = crate::utils::replace_variables(&r.url, &vars);
        r.url = crate::utils::resolve_builtin_variables(&r.url);
        for h in &mut r.headers {
            h.value = crate::utils::replace_variables(&h.value, &vars);
            h.value = crate::utils::resolve_builtin_variables(&h.value);
        }
        for p in &mut r.params {
            p.value = crate::utils::replace_variables(&p.value, &vars);
            p.value = crate::utils::resolve_builtin_variables(&p.value);
        }
        r.body = crate::utils::replace_variables(&r.body, &vars);
        r.body = crate::utils::resolve_builtin_variables(&r.body);
        r
    }

    pub fn open_dialog(&mut self, dtype: DialogType, message: impl Into<String>) {
        self.dialog_type = dtype;
        self.dialog_message = message.into();
        self.dialog_buffer.clear();
        self.dialog_cursor = 0;
        self.input_mode = InputMode::Editing;
    }

    pub fn close_dialog(&mut self) {
        self.dialog_type = DialogType::None;
        self.dialog_buffer.clear();
        self.dialog_cursor = 0;
        self.input_mode = InputMode::Normal;
    }

    pub fn confirm_dialog(&mut self) -> Option<String> {
        let result = if self.dialog_buffer.is_empty() {
            None
        } else {
            Some(self.dialog_buffer.clone())
        };
        self.close_dialog();
        result
    }

    pub fn save_current_request_to_selected_collection(&mut self) {
        // Prompt for name if empty
        if self.current_request.name.is_empty() {
            self.open_dialog(DialogType::RequestName, "Request name:");
            return;
        }
        self.do_save_request();
    }

    pub fn do_save_request(&mut self) {
        // If this request was loaded from an existing source, update it in place
        if let Some((ci, ri)) = self.current_request_source {
            if ci < self.data.collections.len() && ri < self.data.collections[ci].requests.len() {
                let req = self.current_request.clone();
                self.data.collections[ci].requests[ri] = req;
                let _ = self.save();
                self.set_status(format!("Updated request in '{}'", self.data.collections[ci].name));
                self.active_pane = ActivePane::Sidebar;
                return;
            }
        }

        // Otherwise, save as a new request into the selected collection
        if let Some(ci) = self.get_selected_or_parent_collection_index() {
            let req = self.current_request.clone();
            let col_name = self.data.collections[ci].name.clone();
            let col_id = self.data.collections[ci].id.clone();
            self.data.collections[ci].requests.push(req);
            let _ = self.save();
            self.set_status(format!("Saved to '{}'", col_name));

            // Expand collection if collapsed
            if !self.collection_expanded.contains(&col_id) {
                self.collection_expanded.push(col_id);
            }

            // Compute flat sidebar index of the newly added request
            let mut idx = 0;
            for (i, col) in self.data.collections.iter().enumerate() {
                if i == ci {
                    self.sidebar_selected = idx + col.requests.len();
                    break;
                }
                idx += 1; // collection title row
                if self.collection_expanded.contains(&col.id) {
                    idx += col.requests.len();
                }
            }

            self.active_pane = ActivePane::Sidebar;
        } else if !self.data.collections.is_empty() {
            self.set_status("Select a collection first (use ↑/↓ in sidebar)".to_string());
        } else {
            self.set_status("No collections. Press Ctrl+N to create one.".to_string());
        }
    }

    pub fn activate_window_prefix(&mut self) {
        self.pending_window_prefix = true;
        self.pending_window_expiry = self.tick + crate::config::WINDOW_PREFIX_TIMEOUT_TICKS;
    }

    pub fn create_collection(&mut self, name: &str) {
        let col = Collection {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            requests: vec![],
            created_at: chrono::Local::now(),
        };
        self.data.collections.push(col);
        let _ = self.save();
        self.set_status(format!("Created collection '{}'", name));
    }

    pub fn delete_selected_item(&mut self) {
        if self.sidebar_tab != SidebarTab::Collections {
            return;
        }
        let mut idx = 0;
        for (ci, col) in self.data.collections.iter().enumerate() {
            if self.sidebar_selected == idx {
                self.pending_delete = Some(DeleteTarget::Collection(ci));
                self.dialog_option_selected = false;
                self.open_dialog(DialogType::DeleteConfirm, col.name.clone());
                return;
            }
            idx += 1;
            if self.collection_expanded.contains(&col.id) {
                for (ri, req) in col.requests.iter().enumerate() {
                    if self.sidebar_selected == idx {
                        self.pending_delete = Some(DeleteTarget::Request(ci, ri));
                        self.dialog_option_selected = false;
                        self.open_dialog(DialogType::DeleteConfirm, req.name.clone());
                        return;
                    }
                    idx += 1;
                }
            }
        }
    }

    pub fn execute_pending_delete(&mut self) {
        if let Some(target) = self.pending_delete.take() {
            match target {
                DeleteTarget::Collection(ci) => {
                    if ci < self.data.collections.len() {
                        let name = self.data.collections[ci].name.clone();
                        let id = self.data.collections[ci].id.clone();
                        self.data.collections.remove(ci);
                        self.collection_expanded.retain(|x| x != &id);
                        let _ = self.save();
                        self.set_status(format!("Deleted collection '{}'", name));
                        let max = self.sidebar_item_count();
                        if self.sidebar_selected >= max && max > 0 {
                            self.sidebar_selected = max - 1;
                        }
                    }
                }
                DeleteTarget::Request(ci, ri) => {
                    if ci < self.data.collections.len() && ri < self.data.collections[ci].requests.len() {
                        let name = self.data.collections[ci].requests[ri].name.clone();
                        self.data.collections[ci].requests.remove(ri);
                        let _ = self.save();
                        self.set_status(format!("Deleted request '{}'", name));
                        let max = self.sidebar_item_count();
                        if self.sidebar_selected >= max && max > 0 {
                            self.sidebar_selected = max - 1;
                        }
                    }
                }
            }
        }
    }

    /// 搜索请求（按名称和 URL，大小写不敏感）
    pub fn search_requests(&self, query: &str) -> Vec<(usize, usize, String)> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();
        for (ci, col) in self.data.collections.iter().enumerate() {
            for (ri, req) in col.requests.iter().enumerate() {
                if req.name.to_lowercase().contains(&query_lower)
                    || req.url.to_lowercase().contains(&query_lower)
                {
                    results.push((ci, ri, format!("{} {} {}", req.method, req.name, req.url)));
                }
            }
        }
        results
    }

    pub fn export_collection(&mut self, format: &str) -> Option<String> {
        if let Some(ci) = self.get_selected_collection_index() {
            let col = &self.data.collections[ci];
            let result = match format {
                "postman" => crate::export_import::export_collection_postman(col),
                _ => crate::export_import::export_collection_json(col),
            };
            match result {
                Ok(content) => {
                    self.set_status(format!("Exported '{}'", col.name));
                    return Some(content);
                }
                Err(e) => self.set_status(format!("Export failed: {}", e)),
            }
        } else {
            self.set_status("No collection selected");
        }
        None
    }
}
