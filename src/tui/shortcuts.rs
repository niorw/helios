use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::tui::app::{ActivePane, RequestTab, SidebarTab};

/// All possible user actions triggered by keyboard shortcuts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    // Global / window switching
    SwitchPane(u8),
    NextPane,
    PrevPane,
    WindowPrefix,
    Quit,
    SaveRequest,
    ExportCollection,
    NewCollection,
    NewRequest,
    ShowHelp,
    ShowHistory,       // Ctrl+R 显示历史记录
    ShowBookmarks,     // Ctrl+S 书签/收藏夹

    // Sidebar
    SidebarNext,
    SidebarPrev,
    SidebarExpand,
    SidebarCollapse,
    SidebarLoad,
    SidebarDelete,
    SidebarRename,
    SidebarTabCollections,
    SidebarTabEnvironments,

    // URL Bar
    CycleMethodNext,
    CycleMethodPrev,
    EditUrl,
    EditRequestName,
    SendRequest,

    // Request tabs
    SwitchTabParams,
    SwitchTabHeaders,
    SwitchTabBody,
    SwitchTabAuth,
    TabNext,
    TabPrev,
    AddRow,
    DeleteRow,
    ToggleEnabled,
    EditKey,
    EditValue,
    CycleBodyType,
    CycleAuthType,

    // Response
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    ScrollHalfPageUp,
    ScrollHalfPageDown,
    ScrollTop,
    ScrollBottom,
    CopyBody,
    ToggleResponseTab,

    // No-op
    None,
}

/// Parse a key event into an action, considering the current application state.
/// Returns `(action, new_pending_prefix_state)`.
pub fn parse(
    key: KeyEvent,
    active_pane: ActivePane,
    sidebar_tab: SidebarTab,
    request_tab: RequestTab,
    pending_prefix: bool,
) -> (Action, bool) {
    // Window prefix mode takes precedence
    if pending_prefix {
        let action = match key.code {
            KeyCode::Char('1') => Action::SwitchPane(1),
            KeyCode::Char('2') => Action::SwitchPane(2),
            KeyCode::Char('3') => Action::SwitchPane(3),
            KeyCode::Char('4') => Action::SwitchPane(4),
            _ => return (parse_normal(key, active_pane, sidebar_tab, request_tab), false),
        };
        return (action, false);
    }

    // Global shortcuts (available regardless of active pane)
    let global = parse_global(key);
    if global == Action::WindowPrefix {
        return (Action::WindowPrefix, true);
    }
    if global != Action::None {
        return (global, false);
    }

    // Pane-specific shortcuts
    let action = match active_pane {
        ActivePane::Sidebar => parse_sidebar(key, sidebar_tab),
        ActivePane::UrlBar => parse_urlbar(key),
        ActivePane::RequestTabs => parse_request_tabs(key, request_tab),
        ActivePane::Response => parse_response(key),
    };

    (action, false)
}

/// Parse global shortcuts (window switching, quit, save, etc.).
fn parse_global(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::F(1) => Action::SwitchPane(1),
        KeyCode::F(2) => Action::SwitchPane(2),
        KeyCode::F(3) => Action::SwitchPane(3),
        KeyCode::F(4) => Action::SwitchPane(4),
        KeyCode::Char('x') => Action::WindowPrefix,
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                Action::PrevPane
            } else {
                Action::NextPane
            }
        }
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::SaveRequest,
        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Action::ExportCollection
        }
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::ShowHistory,
        KeyCode::Char('?') => Action::ShowHelp,
        _ => Action::None,
    }
}
fn parse_normal(
    key: KeyEvent,
    active_pane: ActivePane,
    sidebar_tab: SidebarTab,
    request_tab: RequestTab,
) -> Action {
    let global = parse_global(key);
    if global != Action::None {
        return global;
    }
    match active_pane {
        ActivePane::Sidebar => parse_sidebar(key, sidebar_tab),
        ActivePane::UrlBar => parse_urlbar(key),
        ActivePane::RequestTabs => parse_request_tabs(key, request_tab),
        ActivePane::Response => parse_response(key),
    }
}

fn parse_sidebar(key: KeyEvent, sidebar_tab: SidebarTab) -> Action {
    match key.code {
        KeyCode::Down => Action::SidebarNext,
        KeyCode::Up => Action::SidebarPrev,
        KeyCode::Right => Action::SidebarExpand,
        KeyCode::Left => Action::SidebarCollapse,
        KeyCode::Enter => Action::SidebarLoad,
        KeyCode::Char('d') if sidebar_tab == SidebarTab::Collections => Action::SidebarDelete,
        KeyCode::Char('n') if sidebar_tab == SidebarTab::Collections => Action::SidebarRename,
        KeyCode::Char('i') if sidebar_tab == SidebarTab::Collections => Action::NewRequest,
        KeyCode::Char('f') if sidebar_tab == SidebarTab::Collections => Action::NewCollection,
        KeyCode::Char('c') => Action::SidebarTabCollections,
        KeyCode::Char('e') => Action::SidebarTabEnvironments,
        _ => Action::None,
    }
}

fn parse_urlbar(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Up => Action::CycleMethodPrev,
        KeyCode::Down => Action::CycleMethodNext,
        KeyCode::Char('u') => Action::EditUrl,
        KeyCode::Char('n') => Action::EditRequestName,
        KeyCode::Enter => Action::SendRequest,
        _ => Action::None,
    }
}

fn parse_request_tabs(key: KeyEvent, request_tab: RequestTab) -> Action {
    match key.code {
        KeyCode::Char('p') => Action::SwitchTabParams,
        KeyCode::Char('h') => Action::SwitchTabHeaders,
        KeyCode::Char('b') => Action::SwitchTabBody,
        KeyCode::Char('a') => Action::SwitchTabAuth,
        KeyCode::Left => Action::TabPrev,
        KeyCode::Right => Action::TabNext,
        KeyCode::Char('i') => Action::AddRow,
        KeyCode::Char('d') => Action::DeleteRow,
        KeyCode::Char(' ') => Action::ToggleEnabled,
        KeyCode::Char('e') => Action::EditKey,
        KeyCode::Char('v') => Action::EditValue,
        KeyCode::Char('t') => match request_tab {
            RequestTab::Body => Action::CycleBodyType,
            RequestTab::Auth => Action::CycleAuthType,
            _ => Action::None,
        },
        _ => Action::None,
    }
}

fn parse_response(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Up => Action::ScrollUp,
        KeyCode::Down => Action::ScrollDown,
        KeyCode::PageUp => Action::PageUp,
        KeyCode::PageDown => Action::PageDown,
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::ScrollHalfPageDown,
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::ScrollHalfPageUp,
        KeyCode::Char('g') => Action::ScrollTop,
        KeyCode::Char('G') => Action::ScrollBottom,
        KeyCode::Char('y') => Action::CopyBody,
        KeyCode::Left | KeyCode::Right => Action::ToggleResponseTab,
        _ => Action::None,
    }
}

// ─── Help text strings ───────────────────────────────────────────────

pub fn help_text(
    active_pane: ActivePane,
    sidebar_tab: SidebarTab,
    request_tab: RequestTab,
    pending_prefix: bool,
) -> &'static str {
    if pending_prefix {
        return "Press 1/2/3/4 to switch pane │ Enter=send │ Ctrl+Q=quit";
    }
    match active_pane {
        ActivePane::Sidebar => {
            if sidebar_tab == SidebarTab::Collections {
                "x+1/2/3/4 switch pane │ Enter load │ d delete │ n rename │ i new req │ f new coll │ →/← expand │ c=coll │ e=env"
            } else {
                "x+1/2/3/4 switch pane │ Enter activate │ ↑/↓ navigate │ c=coll │ e=env"
            }
        }
        ActivePane::UrlBar => "↑/↓ method │ u URL │ n name │ Enter send │ Tab next",
        ActivePane::RequestTabs => match request_tab {
            RequestTab::Params => "i add │ d delete │ Space toggle │ e/v edit │ ←/→ tab",
            RequestTab::Headers => "i add │ d delete │ Space toggle │ e/v cycle │ ←/→ tab",
            RequestTab::Body => "t cycle type │ e edit body │ ←/→ tab",
            RequestTab::Auth => "t cycle type │ e edit token │ ←/→ tab",
        },
        ActivePane::Response => "↑/↓ scroll │ PgUp/PgDn page │ y copy │ ←/→ tab",
    }
}
