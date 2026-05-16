use crate::history::{HistoryManager, HistoryStorage};
use crate::models::{
    AppData, Auth, BodyType, Collection, Folder, HistoryItem, HttpMethod, KeyValue, Request,
    Response,
};
use crate::storage::Storage;
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
    History,       // 历史记录弹窗
    HistorySearch, // 历史搜索弹窗
}

/// Identifies the type of a sidebar item for selection logic.
#[derive(Debug, Clone, PartialEq)]
pub enum SidebarItemType {
    /// A collection header row
    Collection(usize),
    /// A folder row: collection_index, folder_path (indices into nested folders)
    Folder(usize, Vec<usize>),
    /// A request row: collection_index, folder_path, request_index_in_folder
    /// folder_path is empty for root-level requests
    Request(usize, Vec<usize>, usize),
}

#[derive(Debug, Clone)]
pub enum DeleteTarget {
    Collection(usize),
    Request(usize, usize), // collection_index, request_index (root-level)
    FolderRequest(usize, Vec<usize>, usize), // collection_index, folder_path, request_index_in_folder
}

/// Tracks where the currently loaded request came from, for in-place save.
#[derive(Debug, Clone, PartialEq)]
pub enum RequestSource {
    /// Request lives at root of a collection: (collection_index, request_index)
    Root(usize, usize),
    /// Request lives inside a folder: (collection_index, folder_path, request_index_in_folder)
    Folder(usize, Vec<usize>, usize),
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
    pub folder_expanded: Vec<String>,

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

    pub current_request_source: Option<RequestSource>,
    pub pending_delete: Option<DeleteTarget>,

    // History feature
    pub history_manager: HistoryManager,
    pub history_storage: HistoryStorage,
    pub history_selected: usize,
}

// ─── Folder traversal helpers ─────────────────────────────────────────

/// Get a reference to a folder by following the path of indices.
pub fn get_folder_by_path<'a>(folders: &'a [Folder], path: &[usize]) -> Option<&'a Folder> {
    if path.is_empty() {
        return None;
    }
    let first = folders.get(path[0])?;
    let mut current = first;
    for &idx in &path[1..] {
        current = current.folders.get(idx)?;
    }
    Some(current)
}

/// Get a mutable reference to a folder by following the path of indices.
fn get_folder_by_path_mut<'a>(folders: &'a mut [Folder], path: &[usize]) -> Option<&'a mut Folder> {
    if path.is_empty() {
        return None;
    }
    if path.len() == 1 {
        return folders.get_mut(path[0]);
    }
    let first_idx = path[0];
    let remaining = &path[1..];
    let first = folders.get_mut(first_idx)?;
    let mut current = first;
    for &idx in remaining {
        current = current.folders.get_mut(idx)?;
    }
    Some(current)
}

/// Recursively count visible sidebar items within a folder (folders + requests).
fn count_folder_items(folder: &Folder, folder_expanded: &[String]) -> usize {
    let mut count = 1; // the folder itself
    if folder_expanded.contains(&folder.id) {
        for req in &folder.requests {
            count += 1; // each request
            let _ = req; // suppress unused warning
        }
        for sub in &folder.folders {
            count += count_folder_items(sub, folder_expanded);
        }
    }
    count
}

/// Recursively collect sidebar items from folders, building the flat sidebar index.
fn collect_folder_items(
    folder: &Folder,
    ci: usize,
    path: &mut Vec<usize>,
    folder_expanded: &[String],
    items: &mut Vec<SidebarItemType>,
) {
    items.push(SidebarItemType::Folder(ci, path.clone()));
    if folder_expanded.contains(&folder.id) {
        for (ri, _) in folder.requests.iter().enumerate() {
            items.push(SidebarItemType::Request(ci, path.clone(), ri));
        }
        for (fi, sub) in folder.folders.iter().enumerate() {
            path.push(fi);
            collect_folder_items(sub, ci, path, folder_expanded, items);
            path.pop();
        }
    }
}

/// Collect all visible sidebar items for the Collections tab.
pub fn collect_sidebar_items(app: &App) -> Vec<SidebarItemType> {
    let mut items = Vec::new();
    for (ci, col) in app.data.collections.iter().enumerate() {
        items.push(SidebarItemType::Collection(ci));
        if app.collection_expanded.contains(&col.id) {
            // Root-level requests
            for (ri, _) in col.requests.iter().enumerate() {
                items.push(SidebarItemType::Request(ci, Vec::new(), ri));
            }
            // Folders
            for (fi, folder) in col.folders.iter().enumerate() {
                let mut path = vec![fi];
                collect_folder_items(folder, ci, &mut path, &app.folder_expanded, &mut items);
            }
        }
    }
    items
}

/// Get the SidebarItemType at the current sidebar_selected index.
pub fn get_selected_item_type(app: &App) -> Option<SidebarItemType> {
    let items = collect_sidebar_items(app);
    items.into_iter().nth(app.sidebar_selected)
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
            folder_expanded: Vec::new(),
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
            let mk = |name: &str,
                      method: HttpMethod,
                      url: &str,
                      headers: Vec<(&str, &str)>,
                      body: &str,
                      body_type: BodyType|
             -> Request {
                Request {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: name.to_string(),
                    method,
                    url: url.to_string(),
                    headers: headers
                        .into_iter()
                        .map(|(k, v)| KeyValue {
                            key: k.to_string(),
                            value: v.to_string(),
                            enabled: true,
                        })
                        .collect(),
                    params: vec![],
                    body: body.to_string(),
                    body_type,
                    auth: Auth::None,
                    graphql_query: None,
                    graphql_variables: None,
                    form_data: vec![],
                    notes: String::new(),
                }
            };

            app.data.collections = vec![
                Collection {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: "🌐 httpbin.org".to_string(),
                    folders: vec![],
                    requests: [
                        mk("GET 查询参数", HttpMethod::GET, "https://httpbin.org/get?foo=bar&baz=qux", vec![("Accept","application/json")], "", BodyType::None),
                        mk("POST JSON", HttpMethod::POST, "https://httpbin.org/post", vec![("Content-Type","application/json"),("Accept","application/json")], r#"{"username":"helios","password":"demo123"}"#, BodyType::Json),
                        mk("PUT 更新", HttpMethod::PUT, "https://httpbin.org/put", vec![("Content-Type","application/json"),("Accept","application/json")], r#"{"id":1,"name":"updated"}"#, BodyType::Json),
                        mk("PATCH 部分更新", HttpMethod::PATCH, "https://httpbin.org/patch", vec![("Content-Type","application/json"),("Accept","application/json")], r#"{"name":"patched"}"#, BodyType::Json),
                        mk("DELETE 资源", HttpMethod::DELETE, "https://httpbin.org/delete", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET IP 地址", HttpMethod::GET, "https://httpbin.org/ip", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET User-Agent", HttpMethod::GET, "https://httpbin.org/user-agent", vec![("Accept","application/json")], "", BodyType::None),
                        mk("GET Base64 解码", HttpMethod::GET, "https://httpbin.org/base64/SGVsbG8gV29ybGQ=", vec![("Accept","text/plain")], "", BodyType::None),
                    ].to_vec(),
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
                    folders: vec![],
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
                    folders: vec![],
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
                    folders: vec![],
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
                    folders: vec![],
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
                    folders: vec![],
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
                    folders: vec![],
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
                    folders: vec![],
                    created_at: chrono::Local::now(),
                },
            ];
            let _ = app.save();
        }

        Ok(app)
    }

    pub fn save(&self) -> Result<()> {
        self.storage.save(&self.data)?;
        // 同时写入文件系统格式（双写，渐进式迁移）
        if let Ok(file_storage) = crate::file_storage::FileStorage::with_default_path() {
            let _ = file_storage.save_app_data(&self.data);
        }
        Ok(())
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((
            msg.into(),
            self.tick + crate::config::STATUS_MESSAGE_TIMEOUT_TICKS,
        ));
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
                let items = collect_sidebar_items(self);
                items.len()
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
        let item_type = match get_selected_item_type(self) {
            Some(t) => t,
            None => return false,
        };
        match item_type {
            SidebarItemType::Request(ci, ref path, ri) => {
                let req = if path.is_empty() {
                    // Root-level request
                    self.data
                        .collections
                        .get(ci)
                        .and_then(|col| col.requests.get(ri))
                        .cloned()
                } else {
                    // Folder request
                    self.data
                        .collections
                        .get(ci)
                        .and_then(|col| get_folder_by_path(&col.folders, path))
                        .and_then(|folder| folder.requests.get(ri))
                        .cloned()
                };
                if let Some(req) = req {
                    let name = req.name.clone();
                    let source = if path.is_empty() {
                        RequestSource::Root(ci, ri)
                    } else {
                        RequestSource::Folder(ci, path.clone(), ri)
                    };
                    self.load_request(req, Some(source));
                    self.set_status(format!("Loaded: {}", name));
                    return true;
                }
                false
            }
            SidebarItemType::Collection(_) | SidebarItemType::Folder(_, _) => false,
        }
    }

    pub fn get_selected_collection_index(&self) -> Option<usize> {
        if self.sidebar_tab != SidebarTab::Collections {
            return None;
        }
        match get_selected_item_type(self) {
            Some(SidebarItemType::Collection(ci)) => Some(ci),
            _ => None,
        }
    }

    /// Returns the collection index that contains the currently selected sidebar item.
    /// Works whether the selection is on a collection header, folder, or one of its requests.
    pub fn get_selected_or_parent_collection_index(&self) -> Option<usize> {
        if self.sidebar_tab != SidebarTab::Collections {
            return None;
        }
        match get_selected_item_type(self) {
            Some(SidebarItemType::Collection(ci)) => Some(ci),
            Some(SidebarItemType::Folder(ci, _)) => Some(ci),
            Some(SidebarItemType::Request(ci, _, _)) => Some(ci),
            None => None,
        }
    }

    /// Toggle expand/collapse for the currently selected collection or folder.
    pub fn toggle_expand(&mut self) {
        if self.sidebar_tab != SidebarTab::Collections {
            return;
        }
        let item_type = match get_selected_item_type(self) {
            Some(t) => t,
            None => return,
        };
        match item_type {
            SidebarItemType::Collection(ci) => {
                let col_id = self.data.collections.get(ci).map(|c| c.id.clone());
                if let Some(id) = col_id {
                    if self.collection_expanded.contains(&id) {
                        self.collection_expanded.retain(|x| x != &id);
                    } else {
                        self.collection_expanded.push(id);
                    }
                }
            }
            SidebarItemType::Folder(ci, ref path) => {
                let folder_id = self
                    .data
                    .collections
                    .get(ci)
                    .and_then(|col| get_folder_by_path(&col.folders, path))
                    .map(|f| f.id.clone());
                if let Some(id) = folder_id {
                    if self.folder_expanded.contains(&id) {
                        self.folder_expanded.retain(|x| x != &id);
                    } else {
                        self.folder_expanded.push(id);
                    }
                }
            }
            SidebarItemType::Request(_, _, _) => {}
        }
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
            let sel = self.param_list_state.selected().unwrap_or(0).min(max);
            self.param_list_state.select(Some(sel));
        }
    }

    pub fn remove_header(&mut self, idx: usize) {
        if idx < self.current_request.headers.len() {
            self.current_request.headers.remove(idx);
            let max = self.current_request.headers.len().saturating_sub(1);
            let sel = self.header_list_state.selected().unwrap_or(0).min(max);
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
                    self.current_request.auth = Auth::Bearer {
                        token: self.edit_buffer.clone(),
                    };
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
        self.sync_current_request_to_source();
        self.cancel_edit();
    }

    /// Sync the current request back to where it was loaded from.
    fn sync_current_request_to_source(&mut self) {
        if let Some(ref source) = self.current_request_source.clone() {
            match source {
                RequestSource::Root(ci, ri) => {
                    if *ci < self.data.collections.len()
                        && *ri < self.data.collections[*ci].requests.len()
                    {
                        let req = self.current_request.clone();
                        self.data.collections[*ci].requests[*ri] = req;
                        let _ = self.save();
                    }
                }
                RequestSource::Folder(ci, path, ri) => {
                    if *ci < self.data.collections.len() {
                        if let Some(folder) =
                            get_folder_by_path_mut(&mut self.data.collections[*ci].folders, path)
                        {
                            if *ri < folder.requests.len() {
                                let req = self.current_request.clone();
                                folder.requests[*ri] = req;
                                let _ = self.save();
                            }
                        }
                    }
                }
            }
        }
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
        s[..pos]
            .char_indices()
            .last()
            .map(|(idx, _)| idx)
            .unwrap_or(0)
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
            self.dialog_buffer
                .replace_range(prev..self.dialog_cursor, "");
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
                "Content-Type",
                "Accept",
                "Authorization",
                "User-Agent",
                "Cache-Control",
                "X-Request-Id",
                "X-Api-Key",
                "Referer",
                "Origin",
            ];
            let pos = common
                .iter()
                .position(|&k| k == h.key)
                .unwrap_or(common.len() - 1);
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
                "Content-Type" => vec![
                    "application/json",
                    "application/x-www-form-urlencoded",
                    "text/plain",
                    "application/xml",
                    "multipart/form-data",
                ],
                "Accept" => vec!["application/json", "text/html", "application/xml", "*/*"],
                "Authorization" => vec!["Bearer ", "Basic "],
                "Cache-Control" => vec!["no-cache", "no-store", "max-age=0", "must-revalidate"],
                "User-Agent" => vec![
                    crate::config::DEFAULT_USER_AGENT,
                    "Mozilla/5.0",
                    "curl/7.64.1",
                ],
                _ => vec![""],
            };
            if !presets.is_empty() {
                let pos = presets
                    .iter()
                    .position(|&v| h.value == v)
                    .unwrap_or(presets.len() - 1);
                let next = presets[(pos + 1) % presets.len()];
                h.value = next.to_string();
            }
        }
    }

    pub fn cycle_auth_type(&mut self) {
        self.current_request.auth = match self.current_request.auth {
            Auth::None => Auth::Bearer {
                token: String::new(),
            },
            Auth::Bearer { .. } => Auth::Basic {
                username: String::new(),
                password: String::new(),
            },
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

    pub fn load_request(&mut self, req: Request, source: Option<RequestSource>) {
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
        for h in &mut r.headers {
            h.value = crate::utils::replace_variables(&h.value, &vars);
        }
        for p in &mut r.params {
            p.value = crate::utils::replace_variables(&p.value, &vars);
        }
        r.body = crate::utils::replace_variables(&r.body, &vars);
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
        if let Some(ref source) = self.current_request_source.clone() {
            match source {
                RequestSource::Root(ci, ri) => {
                    if *ci < self.data.collections.len()
                        && *ri < self.data.collections[*ci].requests.len()
                    {
                        let req = self.current_request.clone();
                        self.data.collections[*ci].requests[*ri] = req;
                        let _ = self.save();
                        self.set_status(format!(
                            "Updated request in '{}'",
                            self.data.collections[*ci].name
                        ));
                        self.active_pane = ActivePane::Sidebar;
                        return;
                    }
                }
                RequestSource::Folder(ci, path, ri) => {
                    if *ci < self.data.collections.len() {
                        if let Some(folder) =
                            get_folder_by_path_mut(&mut self.data.collections[*ci].folders, path)
                        {
                            if *ri < folder.requests.len() {
                                let req = self.current_request.clone();
                                folder.requests[*ri] = req;
                                let _ = self.save();
                                self.set_status(format!("Updated request in folder"));
                                self.active_pane = ActivePane::Sidebar;
                                return;
                            }
                        }
                    }
                }
            }
        }

        // Otherwise, save as a new request into the selected collection or folder
        let item_type = get_selected_item_type(self);
        if let Some(ci) = self.get_selected_or_parent_collection_index() {
            let req = self.current_request.clone();
            let col_name = self.data.collections[ci].name.clone();
            let col_id = self.data.collections[ci].id.clone();

            // Determine where to save: if a folder is selected, save into it
            match item_type {
                Some(SidebarItemType::Folder(_, ref path)) => {
                    if let Some(folder) =
                        get_folder_by_path_mut(&mut self.data.collections[ci].folders, path)
                    {
                        let folder_name = folder.name.clone();
                        let folder_id = folder.id.clone();
                        folder.requests.push(req);
                        let _ = self.save();
                        self.set_status(format!("Saved to folder '{}'", folder_name));

                        // Expand collection and folder
                        if !self.collection_expanded.contains(&col_id) {
                            self.collection_expanded.push(col_id);
                        }
                        if !self.folder_expanded.contains(&folder_id) {
                            self.folder_expanded.push(folder_id);
                        }
                    } else {
                        // Fallback: save to collection root
                        self.data.collections[ci].requests.push(req);
                        let _ = self.save();
                        self.set_status(format!("Saved to '{}'", col_name));
                    }
                }
                _ => {
                    // Save to collection root
                    self.data.collections[ci].requests.push(req);
                    let _ = self.save();
                    self.set_status(format!("Saved to '{}'", col_name));

                    if !self.collection_expanded.contains(&col_id) {
                        self.collection_expanded.push(col_id);
                    }
                }
            }

            // Recompute sidebar index for newly added item
            let items = collect_sidebar_items(self);
            // The new item is the last Request-type item in this collection
            for (i, item) in items.iter().enumerate().rev() {
                if let SidebarItemType::Request(item_ci, _, _) = item {
                    if *item_ci == ci {
                        self.sidebar_selected = i;
                        break;
                    }
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
            folders: vec![],
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
        let item_type = match get_selected_item_type(self) {
            Some(t) => t,
            None => return,
        };
        match item_type {
            SidebarItemType::Collection(ci) => {
                let name = self
                    .data
                    .collections
                    .get(ci)
                    .map(|c| c.name.clone())
                    .unwrap_or_default();
                self.pending_delete = Some(DeleteTarget::Collection(ci));
                self.dialog_option_selected = false;
                self.open_dialog(DialogType::DeleteConfirm, name);
            }
            SidebarItemType::Request(ci, ref path, ri) => {
                let name = if path.is_empty() {
                    self.data
                        .collections
                        .get(ci)
                        .and_then(|col| col.requests.get(ri))
                        .map(|r| r.name.clone())
                        .unwrap_or_default()
                } else {
                    self.data
                        .collections
                        .get(ci)
                        .and_then(|col| get_folder_by_path(&col.folders, path))
                        .and_then(|folder| folder.requests.get(ri))
                        .map(|r| r.name.clone())
                        .unwrap_or_default()
                };
                if path.is_empty() {
                    self.pending_delete = Some(DeleteTarget::Request(ci, ri));
                } else {
                    self.pending_delete = Some(DeleteTarget::FolderRequest(ci, path.clone(), ri));
                }
                self.dialog_option_selected = false;
                self.open_dialog(DialogType::DeleteConfirm, name);
            }
            SidebarItemType::Folder(_, _) => {
                // For now, don't support deleting folders from sidebar
                self.set_status("Folder deletion not supported yet".to_string());
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
                    if ci < self.data.collections.len()
                        && ri < self.data.collections[ci].requests.len()
                    {
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
                DeleteTarget::FolderRequest(ci, ref path, ri) => {
                    if ci < self.data.collections.len() {
                        if let Some(folder) =
                            get_folder_by_path_mut(&mut self.data.collections[ci].folders, path)
                        {
                            if ri < folder.requests.len() {
                                let name = folder.requests[ri].name.clone();
                                folder.requests.remove(ri);
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
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Folder, HttpMethod, Request};
    use std::collections::HashMap;

    /// Helper to build a test App with no demo data.
    fn make_test_app() -> App {
        App {
            running: true,
            input_mode: InputMode::Normal,
            active_pane: ActivePane::Sidebar,
            sidebar_tab: SidebarTab::Collections,
            request_tab: RequestTab::Headers,
            response_tab: ResponseTab::Body,
            data: AppData::default(),
            storage: crate::storage::Storage::new().unwrap(),
            current_request: Request::default(),
            response: None,
            editing_field: None,
            edit_buffer: String::new(),
            cursor_pos: 0,
            sidebar_selected: 0,
            collection_expanded: Vec::new(),
            folder_expanded: Vec::new(),
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
            history_manager: HistoryManager::default(),
            history_storage: HistoryStorage::new(std::path::PathBuf::from("/tmp/helios_test")),
            history_selected: 0,
        }
    }

    #[test]
    fn test_sidebar_item_count_no_folders() {
        let mut app = make_test_app();
        app.data.collections = vec![
            Collection {
                id: "c1".to_string(),
                name: "Col1".to_string(),
                folders: vec![],
                requests: vec![Request::new("R1", HttpMethod::GET, "http://a.com")],
                created_at: chrono::Local::now(),
            },
            Collection {
                id: "c2".to_string(),
                name: "Col2".to_string(),
                folders: vec![],
                requests: vec![],
                created_at: chrono::Local::now(),
            },
        ];
        // Nothing expanded: 2 items (2 collection headers)
        assert_eq!(app.sidebar_item_count(), 2);

        // Expand c1: 3 items (2 headers + 1 request)
        app.collection_expanded.push("c1".to_string());
        assert_eq!(app.sidebar_item_count(), 3);
    }

    #[test]
    fn test_sidebar_item_count_with_folders() {
        let mut app = make_test_app();
        app.data.collections = vec![Collection {
            id: "c1".to_string(),
            name: "Col1".to_string(),
            folders: vec![Folder {
                id: "f1".to_string(),
                name: "Folder1".to_string(),
                seq: 0,
                variables: HashMap::new(),
                docs: String::new(),
                folders: vec![],
                requests: vec![Request::new("FR1", HttpMethod::POST, "http://b.com")],
                created_at: chrono::Local::now(),
            }],
            requests: vec![Request::new("R1", HttpMethod::GET, "http://a.com")],
            created_at: chrono::Local::now(),
        }];

        // Nothing expanded: 1 item (collection header)
        assert_eq!(app.sidebar_item_count(), 1);

        // Expand collection only: 3 items (col + root req + folder header)
        app.collection_expanded.push("c1".to_string());
        assert_eq!(app.sidebar_item_count(), 3);

        // Expand folder too: 4 items (col + root req + folder + folder req)
        app.folder_expanded.push("f1".to_string());
        assert_eq!(app.sidebar_item_count(), 4);
    }

    #[test]
    fn test_sidebar_item_count_nested_folders() {
        let mut app = make_test_app();
        app.data.collections = vec![Collection {
            id: "c1".to_string(),
            name: "Col1".to_string(),
            folders: vec![Folder {
                id: "f1".to_string(),
                name: "Outer".to_string(),
                seq: 0,
                variables: HashMap::new(),
                docs: String::new(),
                folders: vec![Folder {
                    id: "f2".to_string(),
                    name: "Inner".to_string(),
                    seq: 0,
                    variables: HashMap::new(),
                    docs: String::new(),
                    folders: vec![],
                    requests: vec![Request::new("IR1", HttpMethod::GET, "http://inner.com")],
                    created_at: chrono::Local::now(),
                }],
                requests: vec![Request::new("OR1", HttpMethod::POST, "http://outer.com")],
                created_at: chrono::Local::now(),
            }],
            requests: vec![],
            created_at: chrono::Local::now(),
        }];

        // Expand all
        app.collection_expanded.push("c1".to_string());
        app.folder_expanded.push("f1".to_string());
        app.folder_expanded.push("f2".to_string());
        // col + outer(folder) + outer_req + inner(folder) + inner_req = 5
        assert_eq!(app.sidebar_item_count(), 5);
    }

    #[test]
    fn test_collect_sidebar_items_ordering() {
        let mut app = make_test_app();
        app.data.collections = vec![Collection {
            id: "c1".to_string(),
            name: "Col1".to_string(),
            folders: vec![Folder {
                id: "f1".to_string(),
                name: "Folder1".to_string(),
                seq: 0,
                variables: HashMap::new(),
                docs: String::new(),
                folders: vec![],
                requests: vec![Request::new("FR1", HttpMethod::POST, "http://b.com")],
                created_at: chrono::Local::now(),
            }],
            requests: vec![Request::new("R1", HttpMethod::GET, "http://a.com")],
            created_at: chrono::Local::now(),
        }];
        app.collection_expanded.push("c1".to_string());
        app.folder_expanded.push("f1".to_string());

        let items = collect_sidebar_items(&app);
        assert_eq!(items.len(), 4);
        assert_eq!(items[0], SidebarItemType::Collection(0));
        assert_eq!(items[1], SidebarItemType::Request(0, Vec::new(), 0)); // root request R1
        assert_eq!(items[2], SidebarItemType::Folder(0, vec![0])); // folder f1
        assert_eq!(items[3], SidebarItemType::Request(0, vec![0], 0)); // folder request FR1
    }

    #[test]
    fn test_get_selected_item_type() {
        let mut app = make_test_app();
        app.data.collections = vec![Collection {
            id: "c1".to_string(),
            name: "Col1".to_string(),
            folders: vec![Folder {
                id: "f1".to_string(),
                name: "Folder1".to_string(),
                seq: 0,
                variables: HashMap::new(),
                docs: String::new(),
                folders: vec![],
                requests: vec![Request::new("FR1", HttpMethod::POST, "http://b.com")],
                created_at: chrono::Local::now(),
            }],
            requests: vec![Request::new("R1", HttpMethod::GET, "http://a.com")],
            created_at: chrono::Local::now(),
        }];
        app.collection_expanded.push("c1".to_string());
        app.folder_expanded.push("f1".to_string());

        // idx 0 = collection
        app.sidebar_selected = 0;
        assert_eq!(
            get_selected_item_type(&app),
            Some(SidebarItemType::Collection(0))
        );

        // idx 1 = root request
        app.sidebar_selected = 1;
        assert_eq!(
            get_selected_item_type(&app),
            Some(SidebarItemType::Request(0, Vec::new(), 0))
        );

        // idx 2 = folder
        app.sidebar_selected = 2;
        assert_eq!(
            get_selected_item_type(&app),
            Some(SidebarItemType::Folder(0, vec![0]))
        );

        // idx 3 = folder request
        app.sidebar_selected = 3;
        assert_eq!(
            get_selected_item_type(&app),
            Some(SidebarItemType::Request(0, vec![0], 0))
        );
    }

    #[test]
    fn test_try_load_selected_request_root() {
        let mut app = make_test_app();
        let req = Request::new("R1", HttpMethod::GET, "http://a.com");
        app.data.collections = vec![Collection {
            id: "c1".to_string(),
            name: "Col1".to_string(),
            folders: vec![],
            requests: vec![req.clone()],
            created_at: chrono::Local::now(),
        }];
        app.collection_expanded.push("c1".to_string());

        // Select root request (idx 1)
        app.sidebar_selected = 1;
        assert!(app.try_load_selected_collection_request());
        assert_eq!(app.current_request.name, "R1");
        assert_eq!(app.current_request_source, Some(RequestSource::Root(0, 0)));

        // Select collection header (idx 0) - should return false
        app.sidebar_selected = 0;
        assert!(!app.try_load_selected_collection_request());
    }

    #[test]
    fn test_try_load_selected_request_folder() {
        let mut app = make_test_app();
        let req = Request::new("FR1", HttpMethod::POST, "http://b.com");
        app.data.collections = vec![Collection {
            id: "c1".to_string(),
            name: "Col1".to_string(),
            folders: vec![Folder {
                id: "f1".to_string(),
                name: "Folder1".to_string(),
                seq: 0,
                variables: HashMap::new(),
                docs: String::new(),
                folders: vec![],
                requests: vec![req.clone()],
                created_at: chrono::Local::now(),
            }],
            requests: vec![],
            created_at: chrono::Local::now(),
        }];
        app.collection_expanded.push("c1".to_string());
        app.folder_expanded.push("f1".to_string());

        // Select folder request (idx 2: col=0, folder=1, folder_req=2)
        app.sidebar_selected = 2;
        assert!(app.try_load_selected_collection_request());
        assert_eq!(app.current_request.name, "FR1");
        assert_eq!(
            app.current_request_source,
            Some(RequestSource::Folder(0, vec![0], 0))
        );
    }

    #[test]
    fn test_toggle_expand_collection() {
        let mut app = make_test_app();
        app.data.collections = vec![Collection {
            id: "c1".to_string(),
            name: "Col1".to_string(),
            folders: vec![],
            requests: vec![],
            created_at: chrono::Local::now(),
        }];

        // Not expanded initially
        app.sidebar_selected = 0;
        app.toggle_expand();
        assert!(app.collection_expanded.contains(&"c1".to_string()));

        // Toggle again - should collapse
        app.toggle_expand();
        assert!(!app.collection_expanded.contains(&"c1".to_string()));
    }

    #[test]
    fn test_toggle_expand_folder() {
        let mut app = make_test_app();
        app.data.collections = vec![Collection {
            id: "c1".to_string(),
            name: "Col1".to_string(),
            folders: vec![Folder {
                id: "f1".to_string(),
                name: "Folder1".to_string(),
                seq: 0,
                variables: HashMap::new(),
                docs: String::new(),
                folders: vec![],
                requests: vec![],
                created_at: chrono::Local::now(),
            }],
            requests: vec![],
            created_at: chrono::Local::now(),
        }];
        app.collection_expanded.push("c1".to_string());

        // Select the folder (idx 1)
        app.sidebar_selected = 1;
        app.toggle_expand();
        assert!(app.folder_expanded.contains(&"f1".to_string()));

        // Toggle again
        app.toggle_expand();
        assert!(!app.folder_expanded.contains(&"f1".to_string()));
    }

    #[test]
    fn test_get_folder_by_path() {
        let folders = vec![
            Folder {
                id: "f1".to_string(),
                name: "First".to_string(),
                seq: 0,
                variables: HashMap::new(),
                docs: String::new(),
                folders: vec![Folder {
                    id: "f1_1".to_string(),
                    name: "Nested".to_string(),
                    seq: 0,
                    variables: HashMap::new(),
                    docs: String::new(),
                    folders: vec![],
                    requests: vec![],
                    created_at: chrono::Local::now(),
                }],
                requests: vec![],
                created_at: chrono::Local::now(),
            },
            Folder {
                id: "f2".to_string(),
                name: "Second".to_string(),
                seq: 1,
                variables: HashMap::new(),
                docs: String::new(),
                folders: vec![],
                requests: vec![],
                created_at: chrono::Local::now(),
            },
        ];

        assert_eq!(get_folder_by_path(&folders, &[0]).unwrap().name, "First");
        assert_eq!(get_folder_by_path(&folders, &[1]).unwrap().name, "Second");
        assert_eq!(
            get_folder_by_path(&folders, &[0, 0]).unwrap().name,
            "Nested"
        );
        assert!(get_folder_by_path(&folders, &[]).is_none());
        assert!(get_folder_by_path(&folders, &[5]).is_none());
    }

    #[test]
    fn test_delete_folder_request() {
        let mut app = make_test_app();
        let req = Request::new("FR1", HttpMethod::POST, "http://b.com");
        app.data.collections = vec![Collection {
            id: "c1".to_string(),
            name: "Col1".to_string(),
            folders: vec![Folder {
                id: "f1".to_string(),
                name: "Folder1".to_string(),
                seq: 0,
                variables: HashMap::new(),
                docs: String::new(),
                folders: vec![],
                requests: vec![req],
                created_at: chrono::Local::now(),
            }],
            requests: vec![],
            created_at: chrono::Local::now(),
        }];
        app.collection_expanded.push("c1".to_string());
        app.folder_expanded.push("f1".to_string());

        // Select folder request (idx 2)
        app.sidebar_selected = 2;
        app.delete_selected_item();
        assert!(app.pending_delete.is_some());
        match app.pending_delete {
            Some(DeleteTarget::FolderRequest(0, ref path, 0)) => {
                assert_eq!(path, &vec![0]);
            }
            other => panic!("Expected FolderRequest, got {:?}", other),
        }

        app.execute_pending_delete();
        assert_eq!(app.data.collections[0].folders[0].requests.len(), 0);
    }

    #[test]
    fn test_sync_current_request_to_source_root() {
        let mut app = make_test_app();
        app.data.collections = vec![Collection {
            id: "c1".to_string(),
            name: "Col1".to_string(),
            folders: vec![],
            requests: vec![Request::new("R1", HttpMethod::GET, "http://a.com")],
            created_at: chrono::Local::now(),
        }];
        app.current_request = Request::new("R1-Modified", HttpMethod::POST, "http://b.com");
        app.current_request_source = Some(RequestSource::Root(0, 0));

        app.sync_current_request_to_source();
        assert_eq!(app.data.collections[0].requests[0].name, "R1-Modified");
        assert_eq!(app.data.collections[0].requests[0].method, HttpMethod::POST);
    }

    #[test]
    fn test_sync_current_request_to_source_folder() {
        let mut app = make_test_app();
        app.data.collections = vec![Collection {
            id: "c1".to_string(),
            name: "Col1".to_string(),
            folders: vec![Folder {
                id: "f1".to_string(),
                name: "Folder1".to_string(),
                seq: 0,
                variables: HashMap::new(),
                docs: String::new(),
                folders: vec![],
                requests: vec![Request::new("FR1", HttpMethod::GET, "http://a.com")],
                created_at: chrono::Local::now(),
            }],
            requests: vec![],
            created_at: chrono::Local::now(),
        }];
        app.current_request = Request::new("FR1-Modified", HttpMethod::PUT, "http://c.com");
        app.current_request_source = Some(RequestSource::Folder(0, vec![0], 0));

        app.sync_current_request_to_source();
        assert_eq!(
            app.data.collections[0].folders[0].requests[0].name,
            "FR1-Modified"
        );
        assert_eq!(
            app.data.collections[0].folders[0].requests[0].method,
            HttpMethod::PUT
        );
    }

    #[test]
    fn test_request_source_equality() {
        assert_eq!(RequestSource::Root(0, 1), RequestSource::Root(0, 1));
        assert_ne!(RequestSource::Root(0, 1), RequestSource::Root(0, 2));
        assert_eq!(
            RequestSource::Folder(0, vec![0], 1),
            RequestSource::Folder(0, vec![0], 1)
        );
        assert_ne!(
            RequestSource::Root(0, 1),
            RequestSource::Folder(0, vec![], 1)
        );
    }
}
