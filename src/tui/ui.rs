use crate::config;
use crate::models::{Auth, Folder, HttpMethod};
use crate::tui::app::{
    ActivePane, App, DialogType, EditingField, InputMode, RequestTab, SidebarItemType, SidebarTab,
};
use chrono::{DateTime, Local};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table, Wrap,
    },
    Frame,
};

// ─── Theme ───────────────────────────────────────────────────────────
const BG: Color = Color::Rgb(20, 20, 30);
const SURFACE: Color = Color::Rgb(30, 30, 45);
const BORDER: Color = Color::Rgb(60, 60, 90);
const BORDER_ACTIVE: Color = Color::Rgb(140, 100, 255); // purple
const TEXT: Color = Color::Rgb(220, 220, 240);
const TEXT_DIM: Color = Color::Rgb(120, 120, 150);
const ACCENT: Color = Color::Rgb(180, 140, 255);
const SUCCESS: Color = Color::Rgb(80, 250, 123);
const WARN: Color = Color::Rgb(255, 184, 108);
const ERROR: Color = Color::Rgb(255, 100, 100);
const METHOD_GET: Color = Color::Rgb(80, 250, 123);
const METHOD_POST: Color = Color::Rgb(255, 184, 108);
const METHOD_PUT: Color = Color::Rgb(100, 200, 255);
const METHOD_DELETE: Color = Color::Rgb(255, 100, 100);
const METHOD_PATCH: Color = Color::Rgb(255, 120, 200);
const METHOD_OTHER: Color = Color::Rgb(150, 150, 180);

fn block(title: impl Into<String>, active: bool) -> Block<'static> {
    Block::default()
        .title(Span::styled(
            title.into(),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if active {
            Style::default().fg(BORDER_ACTIVE)
        } else {
            Style::default().fg(BORDER)
        })
        .style(Style::default().bg(SURFACE))
}

fn method_color(m: &HttpMethod) -> Color {
    match m {
        HttpMethod::GET => METHOD_GET,
        HttpMethod::POST => METHOD_POST,
        HttpMethod::PUT => METHOD_PUT,
        HttpMethod::DELETE => METHOD_DELETE,
        HttpMethod::PATCH => METHOD_PATCH,
        _ => METHOD_OTHER,
    }
}

// ─── Main Draw ───────────────────────────────────────────────────────
pub fn draw(f: &mut Frame, app: &mut App) {
    let full = f.size();
    f.render_widget(Paragraph::new("").style(Style::default().bg(BG)), full);

    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(full);

    draw_title_bar(f, app, main[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(36), Constraint::Min(0)])
        .split(main[1]);

    draw_sidebar(f, app, body[0]);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Percentage(44),
            Constraint::Percentage(50),
        ])
        .split(body[1]);

    draw_urlbar(f, app, right[0]);
    draw_request_tabs(f, app, right[1]);
    draw_response(f, app, right[2]);

    draw_status_bar(f, app, main[2]);

    if app.input_mode == InputMode::Editing && app.editing_field.is_some() {
        draw_edit_popup(f, app, right[1]);
    }
    if app.dialog_type != DialogType::None {
        draw_dialog(f, app, full);
    }
    if app.loading {
        draw_loading(f, full);
    }
}

// ─── Title Bar ───────────────────────────────────────────────────────
fn draw_title_bar(f: &mut Frame, _app: &App, area: Rect) {
    let title = Line::from(vec![
        Span::styled(" ⚡ ", Style::default().fg(ACCENT)),
        Span::styled(
            config::APP_NAME_DISPLAY,
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  v", Style::default().fg(TEXT_DIM)),
        Span::styled(config::APP_VERSION, Style::default().fg(TEXT_DIM)),
    ]);
    let bar = Paragraph::new(title).style(Style::default().bg(SURFACE));
    f.render_widget(bar, area);
}

// ─── Sidebar ─────────────────────────────────────────────────────────
fn draw_sidebar(f: &mut Frame, app: &mut App, area: Rect) {
    let b = block(" [1] Navigator ", app.active_pane == ActivePane::Sidebar);
    f.render_widget(b, area);
    let inner = area.inner(&Margin {
        horizontal: 1,
        vertical: 1,
    });

    let tab_labels = vec![" Collections ", " Environments "];
    let tabs = ratatui::widgets::Tabs::new(tab_labels)
        .select(match app.sidebar_tab {
            SidebarTab::Collections => 0,
            SidebarTab::Environments => 1,
        })
        .highlight_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
        .divider(Span::styled("│", Style::default().fg(BORDER)));
    f.render_widget(tabs, Rect { height: 1, ..inner });

    let items: Vec<ListItem> = match app.sidebar_tab {
        SidebarTab::Collections => {
            let sidebar_items = crate::tui::app::collect_sidebar_items(app);
            let mut items = vec![];
            for (idx, item_type) in sidebar_items.iter().enumerate() {
                let sel = app.sidebar_selected == idx;
                let style = if sel {
                    Style::default()
                        .bg(BORDER_ACTIVE)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(TEXT)
                };
                match item_type {
                    SidebarItemType::Collection(ci) => {
                        let col = &app.data.collections[*ci];
                        let expanded = app.collection_expanded.contains(&col.id);
                        let icon = if expanded { "▼" } else { "▶" };
                        items.push(ListItem::new(format!("{} {}", icon, col.name)).style(style));
                    }
                    SidebarItemType::Folder(ci, path) => {
                        let col = &app.data.collections[*ci];
                        let folder = crate::tui::app::get_folder_by_path(&col.folders, path);
                        let depth = path.len();
                        let indent = "  │ ".repeat(depth);
                        if let Some(folder) = folder {
                            let expanded = app.folder_expanded.contains(&folder.id);
                            let icon = if expanded { "▼" } else { "▶" };
                            items.push(
                                ListItem::new(format!("{}{}📁 {}", indent, icon, folder.name))
                                    .style(style),
                            );
                        }
                    }
                    SidebarItemType::Request(ci, path, ri) => {
                        let col = &app.data.collections[*ci];
                        let req = if path.is_empty() {
                            col.requests.get(*ri)
                        } else {
                            let folder = crate::tui::app::get_folder_by_path(&col.folders, path);
                            folder.and_then(|f| f.requests.get(*ri))
                        };
                        let depth = path.len();
                        let indent = "  │ ".repeat(depth);
                        if let Some(req) = req {
                            let sel_style = if sel {
                                Style::default().bg(BORDER_ACTIVE).fg(Color::Black)
                            } else {
                                Style::default().fg(TEXT_DIM)
                            };
                            let mcolor = method_color(&req.method);
                            let line = Line::from(vec![
                                Span::raw(format!("{}  ├─ ", indent)),
                                Span::styled(
                                    format!("{:6}", req.method.to_string()),
                                    Style::default().fg(mcolor),
                                ),
                                Span::raw(format!(" {}", crate::utils::truncate(&req.name, 22))),
                            ]);
                            items.push(ListItem::new(line).style(sel_style));
                        }
                    }
                }
            }
            if items.is_empty() {
                items.push(
                    ListItem::new("No collections — press f").style(Style::default().fg(TEXT_DIM)),
                );
            }
            items
        }
        SidebarTab::Environments => app
            .data
            .environments
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let sel = app.sidebar_selected == i;
                let style = if sel {
                    Style::default()
                        .bg(BORDER_ACTIVE)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(TEXT)
                };
                let active = app.data.active_env_id.as_ref() == Some(&e.id);
                let prefix = if active { "●" } else { "○" };
                ListItem::new(format!(
                    "{} {} ({} vars)",
                    prefix,
                    e.name,
                    e.variables.len()
                ))
                .style(style)
            })
            .collect(),
    };

    let info_height: u16 = if app.sidebar_tab == SidebarTab::Environments {
        6.min(inner.height.saturating_sub(3))
    } else {
        0
    };

    let list_needed = items.len() as u16;
    let max_list_height = inner.height.saturating_sub(2 + info_height);
    let list_height = if app.sidebar_tab == SidebarTab::Environments {
        list_needed.min(max_list_height)
    } else {
        max_list_height
    };

    let list_area = Rect {
        x: inner.x,
        y: inner.y + 2,
        width: inner.width,
        height: list_height,
    };

    let list = List::new(items).highlight_symbol("");
    f.render_widget(list, list_area);

    // Environment info panel
    if app.sidebar_tab == SidebarTab::Environments && info_height > 0 {
        let info_area = Rect {
            x: inner.x,
            y: list_area.y + list_area.height,
            width: inner.width,
            height: info_height,
        };

        let data_path = app.storage.data_dir().join(crate::config::DATA_FILE_NAME);
        let data_path_str = data_path.to_string_lossy();

        let active_env = app
            .data
            .active_env_id
            .as_ref()
            .and_then(|id| app.data.environments.iter().find(|e| &e.id == id));
        let active_name = active_env.map(|e| e.name.as_str()).unwrap_or("none");

        let selected_env = app.data.environments.get(app.sidebar_selected);

        let mut info_lines: Vec<Line> = vec![
            Line::from(vec![
                Span::styled("Data ", Style::default().fg(ACCENT)),
                Span::styled(data_path_str.to_string(), Style::default().fg(TEXT_DIM)),
            ]),
            Line::from(vec![
                Span::styled("Active ", Style::default().fg(ACCENT)),
                Span::styled(active_name, Style::default().fg(TEXT)),
            ]),
        ];

        if let Some(env) = selected_env {
            info_lines.push(Line::from(Span::styled(
                "─".repeat(inner.width as usize),
                Style::default().fg(BORDER),
            )));
            if env.variables.is_empty() {
                info_lines.push(Line::from(Span::styled(
                    "No variables",
                    Style::default().fg(TEXT_DIM),
                )));
            } else {
                for (k, v) in env.variables.iter().take(3) {
                    info_lines.push(Line::from(vec![
                        Span::styled(format!("{}=", k), Style::default().fg(TEXT_DIM)),
                        Span::styled(v.to_string(), Style::default().fg(TEXT)),
                    ]));
                }
            }
        }

        let info_para = Paragraph::new(info_lines).wrap(Wrap { trim: true });
        f.render_widget(info_para, info_area);
    }
}

// ─── URL Bar ─────────────────────────────────────────────────────────
fn draw_urlbar(f: &mut Frame, app: &mut App, area: Rect) {
    let b = block(" [2] Request ", app.active_pane == ActivePane::UrlBar);
    f.render_widget(b, area);
    let inner = area.inner(&Margin {
        horizontal: 2,
        vertical: 1,
    });

    // Name row
    let name_area = Rect { height: 1, ..inner };
    let name_is_editing = app.input_mode == InputMode::Editing
        && app.editing_field == Some(EditingField::RequestName);
    let name_text = if name_is_editing {
        format!("{}▌", app.edit_buffer)
    } else if app.current_request.name.is_empty() {
        "(unnamed)".to_string()
    } else {
        app.current_request.name.clone()
    };
    let name_style = if name_is_editing {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    };
    let name_para = Paragraph::new(name_text)
        .style(name_style)
        .alignment(Alignment::Center);
    f.render_widget(name_para, name_area);

    // Method + URL + Send row
    let action_area = Rect {
        y: inner.y + 1,
        height: inner.height.saturating_sub(1),
        ..inner
    };
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(10),
            Constraint::Min(20),
            Constraint::Length(10),
        ])
        .split(action_area);

    // Method badge
    let mc = method_color(&app.current_request.method);
    let method_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(mc))
        .style(Style::default().bg(SURFACE));
    let method_text = Paragraph::new(format!(" {} ", app.current_request.method))
        .style(Style::default().fg(mc).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(method_block);
    f.render_widget(method_text, top[0]);

    // URL input
    let url_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(
            if app.input_mode == InputMode::Editing && app.editing_field == Some(EditingField::Url)
            {
                Style::default().fg(ACCENT)
            } else {
                Style::default().fg(BORDER)
            },
        )
        .style(Style::default().bg(SURFACE));
    let url_text =
        if app.input_mode == InputMode::Editing && app.editing_field == Some(EditingField::Url) {
            format!("{}▌", app.edit_buffer)
        } else {
            app.current_request.url.clone()
        };
    let url_para = Paragraph::new(url_text)
        .block(url_block)
        .style(Style::default().fg(TEXT));
    f.render_widget(url_para, top[1]);

    // Send button
    let send_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(SUCCESS))
        .style(Style::default().bg(SURFACE));
    let send_btn = Paragraph::new(" SEND ")
        .style(
            Style::default()
                .fg(Color::Black)
                .bg(SUCCESS)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(send_block);
    f.render_widget(send_btn, top[2]);
}

// ─── Request Tabs ────────────────────────────────────────────────────
fn draw_request_tabs(f: &mut Frame, app: &mut App, area: Rect) {
    let b = block(" [3] Payload ", app.active_pane == ActivePane::RequestTabs);
    f.render_widget(b, area);
    let inner = area.inner(&Margin {
        horizontal: 1,
        vertical: 1,
    });

    let tab_labels = vec![" Params ", " Headers ", " Body ", " Auth "];
    let tabs = ratatui::widgets::Tabs::new(tab_labels)
        .select(match app.request_tab {
            RequestTab::Params => 0,
            RequestTab::Headers => 1,
            RequestTab::Body => 2,
            RequestTab::Auth => 3,
        })
        .highlight_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
        .divider(Span::styled("│", Style::default().fg(BORDER)));
    f.render_widget(tabs, Rect { height: 1, ..inner });

    let content = Rect {
        x: inner.x,
        y: inner.y + 2,
        width: inner.width,
        height: inner.height.saturating_sub(2),
    };

    match app.request_tab {
        RequestTab::Params => draw_params_table(f, app, content),
        RequestTab::Headers => draw_headers_table(f, app, content),
        RequestTab::Body => draw_body_editor(f, app, content),
        RequestTab::Auth => draw_auth_editor(f, app, content),
    }
}

fn draw_params_table(f: &mut Frame, app: &mut App, area: Rect) {
    let rows: Vec<Row> = app
        .current_request
        .params
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let sel = app.param_list_state.selected() == Some(i);
            let style = if sel {
                Style::default().bg(BORDER_ACTIVE).fg(Color::Black)
            } else {
                Style::default().fg(TEXT)
            };
            let enabled = if p.enabled {
                Span::styled(" ✓ ", Style::default().fg(SUCCESS))
            } else {
                Span::styled("   ", Style::default().fg(TEXT_DIM))
            };
            Row::new(vec![
                Cell::from(Span::styled(
                    p.key.clone(),
                    Style::default().fg(if p.enabled { TEXT } else { TEXT_DIM }),
                )),
                Cell::from(Span::styled(
                    p.value.clone(),
                    Style::default().fg(if p.enabled { TEXT } else { TEXT_DIM }),
                )),
                Cell::from(enabled),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(40),
            Constraint::Percentage(45),
            Constraint::Length(5),
        ],
    )
    .header(
        Row::new(vec![
            Cell::from(Span::styled(
                "Key",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Value",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "En",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )),
        ])
        .style(Style::default().bg(SURFACE)),
    )
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(BORDER),
    );
    f.render_stateful_widget(table, area, &mut app.param_list_state);
}

fn draw_headers_table(f: &mut Frame, app: &mut App, area: Rect) {
    let rows: Vec<Row> = app
        .current_request
        .headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let sel = app.header_list_state.selected() == Some(i);
            let style = if sel {
                Style::default().bg(BORDER_ACTIVE).fg(Color::Black)
            } else {
                Style::default().fg(TEXT)
            };
            let enabled = if h.enabled {
                Span::styled(" ✓ ", Style::default().fg(SUCCESS))
            } else {
                Span::styled("   ", Style::default().fg(TEXT_DIM))
            };
            Row::new(vec![
                Cell::from(Span::styled(
                    h.key.clone(),
                    Style::default().fg(if h.enabled { TEXT } else { TEXT_DIM }),
                )),
                Cell::from(Span::styled(
                    h.value.clone(),
                    Style::default().fg(if h.enabled { TEXT } else { TEXT_DIM }),
                )),
                Cell::from(enabled),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(40),
            Constraint::Percentage(45),
            Constraint::Length(5),
        ],
    )
    .header(
        Row::new(vec![
            Cell::from(Span::styled(
                "Key",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Value",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "En",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )),
        ])
        .style(Style::default().bg(SURFACE)),
    )
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(BORDER),
    );
    f.render_stateful_widget(table, area, &mut app.header_list_state);
}

fn draw_body_editor(f: &mut Frame, app: &mut App, area: Rect) {
    let type_label = format!(" {} (press 't' to cycle) ", app.current_request.body_type);
    let block = Block::default()
        .title(Span::styled(type_label, Style::default().fg(ACCENT)))
        .borders(Borders::TOP)
        .border_style(BORDER);

    let text =
        if app.input_mode == InputMode::Editing && app.editing_field == Some(EditingField::Body) {
            format!("{}▌", app.edit_buffer)
        } else {
            app.current_request.body.clone()
        };

    let paragraph = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(TEXT).bg(SURFACE))
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn draw_auth_editor(f: &mut Frame, app: &mut App, area: Rect) {
    let content = match &app.current_request.auth {
        Auth::None => Text::from(vec![
            Line::from(Span::styled(
                "No authentication configured.",
                Style::default().fg(TEXT_DIM),
            )),
            Line::from(Span::styled(
                "Press 'e' to set a Bearer token.",
                Style::default().fg(TEXT_DIM),
            )),
        ]),
        Auth::Bearer { token } => {
            let text = if app.input_mode == InputMode::Editing
                && app.editing_field == Some(EditingField::AuthToken)
            {
                let before = &app.edit_buffer[..app.cursor_pos.min(app.edit_buffer.len())];
                let after = &app.edit_buffer[app.cursor_pos.min(app.edit_buffer.len())..];
                format!("{}▌{}", before, after)
            } else {
                token.clone()
            };
            Text::from(vec![
                Line::from(vec![
                    Span::styled("Type: ", Style::default().fg(ACCENT)),
                    Span::styled("Bearer", Style::default().fg(TEXT)),
                ]),
                Line::from(vec![
                    Span::styled("Token: ", Style::default().fg(ACCENT)),
                    Span::styled(text, Style::default().fg(TEXT)),
                ]),
            ])
        }
        Auth::Basic { username, password } => {
            let user_text = if app.input_mode == InputMode::Editing
                && app.editing_field == Some(EditingField::AuthUsername)
            {
                let before = &app.edit_buffer[..app.cursor_pos.min(app.edit_buffer.len())];
                let after = &app.edit_buffer[app.cursor_pos.min(app.edit_buffer.len())..];
                format!("{}▌{}", before, after)
            } else {
                username.clone()
            };
            let pass_text = if app.input_mode == InputMode::Editing
                && app.editing_field == Some(EditingField::AuthPassword)
            {
                let before = &app.edit_buffer[..app.cursor_pos.min(app.edit_buffer.len())];
                let after = &app.edit_buffer[app.cursor_pos.min(app.edit_buffer.len())..];
                format!("{}▌{}", before, after)
            } else {
                "*".repeat(password.len().min(20))
            };
            Text::from(vec![
                Line::from(vec![
                    Span::styled("Type: ", Style::default().fg(ACCENT)),
                    Span::styled("Basic", Style::default().fg(TEXT)),
                ]),
                Line::from(vec![
                    Span::styled("User: ", Style::default().fg(ACCENT)),
                    Span::styled(user_text, Style::default().fg(TEXT)),
                ]),
                Line::from(vec![
                    Span::styled("Pass: ", Style::default().fg(ACCENT)),
                    Span::styled(pass_text, Style::default().fg(TEXT)),
                ]),
            ])
        }
    };
    let p = Paragraph::new(content).style(Style::default().fg(TEXT).bg(SURFACE));
    f.render_widget(p, area);
}

// ─── Response ────────────────────────────────────────────────────────
fn draw_response(f: &mut Frame, app: &mut App, area: Rect) {
    let b = block(" [4] Response ", app.active_pane == ActivePane::Response);
    f.render_widget(b, area);
    let inner = area.inner(&Margin {
        horizontal: 1,
        vertical: 1,
    });

    if let Some(resp) = &app.response {
        // Response sub-tabs
        let tab_labels = vec![" Body ", " Headers "];
        let tabs = ratatui::widgets::Tabs::new(tab_labels)
            .select(match app.response_tab {
                crate::tui::app::ResponseTab::Body => 0,
                crate::tui::app::ResponseTab::Headers => 1,
            })
            .highlight_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
            .divider(Span::styled("│", Style::default().fg(BORDER)));
        f.render_widget(tabs, Rect { height: 1, ..inner });

        let status_color = if resp.status < 200 {
            TEXT_DIM
        } else if resp.status < 300 {
            SUCCESS
        } else if resp.status < 400 {
            WARN
        } else {
            ERROR
        };

        let status_line = Line::from(vec![
            Span::styled("Status ", Style::default().fg(ACCENT)),
            Span::styled(
                format!("{} {}", resp.status, resp.status_text),
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  │  ", Style::default().fg(BORDER)),
            Span::styled("Time ", Style::default().fg(ACCENT)),
            Span::styled(format!("{}ms", resp.duration_ms), Style::default().fg(TEXT)),
            Span::styled("  │  ", Style::default().fg(BORDER)),
            Span::styled("Size ", Style::default().fg(ACCENT)),
            Span::styled(
                format!("{} bytes", resp.body.len()),
                Style::default().fg(TEXT),
            ),
        ]);
        let status_para = Paragraph::new(status_line);
        f.render_widget(
            status_para,
            Rect {
                x: inner.x,
                y: inner.y + 1,
                width: inner.width,
                height: 1,
            },
        );

        let content_area = Rect {
            x: inner.x,
            y: inner.y + 3,
            width: inner.width,
            height: inner.height.saturating_sub(3),
        };

        match app.response_tab {
            crate::tui::app::ResponseTab::Body => {
                let formatted = crate::utils::format_json(&resp.body);
                let text = highlight_json(&formatted);
                let total_lines = text.lines.len();
                let max_scroll = total_lines.saturating_sub(content_area.height as usize);
                let start = (app.response_scroll.1 as usize).min(max_scroll);
                app.response_scroll.1 = start as u16;

                let visible_lines: Vec<Line> = text
                    .lines
                    .into_iter()
                    .skip(start)
                    .take(content_area.height as usize)
                    .collect();
                let paragraph =
                    Paragraph::new(Text::from(visible_lines)).style(Style::default().bg(SURFACE));
                f.render_widget(paragraph, content_area);

                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("▲"))
                    .end_symbol(Some("▼"))
                    .track_symbol(Some("│"))
                    .thumb_symbol("█");
                let mut state = ScrollbarState::new(max_scroll).position(start);
                f.render_stateful_widget(
                    scrollbar,
                    content_area.inner(&Margin {
                        horizontal: 0,
                        vertical: 0,
                    }),
                    &mut state,
                );
            }
            crate::tui::app::ResponseTab::Headers => {
                let mut header_entries: Vec<_> = resp.headers.iter().collect();
                header_entries.sort_by(|a, b| a.0.cmp(b.0));
                let header_lines: Vec<Line> = header_entries
                    .iter()
                    .map(|(k, v)| {
                        Line::from(vec![
                            Span::styled(format!("{}: ", k), Style::default().fg(ACCENT)),
                            Span::styled((*v).clone(), Style::default().fg(TEXT)),
                        ])
                    })
                    .collect();
                let total_lines = header_lines.len();
                let max_scroll = total_lines.saturating_sub(content_area.height as usize);
                let start = (app.response_scroll.1 as usize).min(max_scroll);
                app.response_scroll.1 = start as u16;

                let visible_lines: Vec<Line> = header_lines
                    .into_iter()
                    .skip(start)
                    .take(content_area.height as usize)
                    .collect();
                let paragraph =
                    Paragraph::new(Text::from(visible_lines)).style(Style::default().bg(SURFACE));
                f.render_widget(paragraph, content_area);

                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("▲"))
                    .end_symbol(Some("▼"))
                    .track_symbol(Some("│"))
                    .thumb_symbol("█");
                let mut state = ScrollbarState::new(max_scroll).position(start);
                f.render_stateful_widget(
                    scrollbar,
                    content_area.inner(&Margin {
                        horizontal: 0,
                        vertical: 0,
                    }),
                    &mut state,
                );
            }
        }
    } else {
        let p = Paragraph::new("No response yet. Press Enter to send request.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(TEXT_DIM));
        f.render_widget(p, inner);
    }
}

// ─── JSON Highlight ──────────────────────────────────────────────────
fn highlight_json(text: &str) -> Text<'static> {
    let mut lines = vec![];
    for line in text.lines() {
        let mut spans = vec![];
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '"' {
                let mut s = String::from(c);
                while let Some(ch) = chars.next() {
                    s.push(ch);
                    if ch == '\\' {
                        if let Some(next_ch) = chars.next() {
                            s.push(next_ch);
                        }
                    } else if ch == '"' {
                        break;
                    }
                }
                spans.push(Span::styled(
                    s,
                    Style::default().fg(Color::Rgb(150, 255, 180)),
                ));
            } else if c.is_numeric()
                || (c == '-' && chars.peek().map(|&p| p.is_numeric()).unwrap_or(false))
            {
                let mut s = String::from(c);
                while let Some(&ch) = chars.peek() {
                    if ch.is_numeric() || ch == '.' {
                        s.push(ch);
                        chars.next();
                    } else {
                        break;
                    }
                }
                spans.push(Span::styled(
                    s,
                    Style::default().fg(Color::Rgb(255, 220, 120)),
                ));
            } else if c.is_alphabetic() {
                let mut s = String::from(c);
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphabetic() {
                        s.push(ch);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let color = match s.as_str() {
                    "true" | "false" => Color::Rgb(255, 120, 200),
                    "null" => Color::Rgb(150, 150, 180),
                    _ => TEXT,
                };
                spans.push(Span::styled(s, Style::default().fg(color)));
            } else if "{}[],:".contains(c) {
                spans.push(Span::styled(
                    c.to_string(),
                    Style::default().fg(Color::Rgb(100, 100, 130)),
                ));
            } else {
                spans.push(Span::raw(c.to_string()));
            }
        }
        lines.push(Line::from(spans));
    }
    Text::from(lines)
}

// ─── Status Bar ──────────────────────────────────────────────────────
fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    // Left: status / message
    let (msg, style) = if app.loading {
        (
            "⚡ Sending...".to_string(),
            Style::default()
                .bg(WARN)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
    } else if let Some((msg, _)) = &app.status_message {
        (
            msg.clone(),
            Style::default().bg(BORDER_ACTIVE).fg(Color::Black),
        )
    } else {
        let info = format!(
            "{} │ {} │ {}p │ {}h │ {}",
            app.current_request.method,
            match app.sidebar_tab {
                SidebarTab::Collections => "Coll",
                SidebarTab::Environments => "Env",
            },
            app.current_request.params.len(),
            app.current_request.headers.len(),
            if app.data.active_env_id.is_some() {
                "Env:ON"
            } else {
                "Env:OFF"
            }
        );
        (info, Style::default().bg(SURFACE).fg(TEXT_DIM))
    };
    let paragraph = Paragraph::new(msg).style(style);
    f.render_widget(paragraph, cols[0]);

    // Right: context-aware shortcuts
    let shortcuts = crate::tui::shortcuts::help_text(
        app.active_pane,
        app.sidebar_tab,
        app.request_tab,
        app.pending_window_prefix,
    );
    let help_line = Line::from(vec![
        Span::styled("⌨ ", Style::default().fg(ACCENT)),
        Span::styled(shortcuts, Style::default().fg(TEXT_DIM)),
    ]);
    let help_para = Paragraph::new(help_line).alignment(Alignment::Right);
    f.render_widget(help_para, cols[1]);
}

// ─── Edit Popup ──────────────────────────────────────────────────────
fn draw_edit_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(70, 30, area);
    let block = Block::default()
        .title(Span::styled(
            " Edit ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(BG));
    f.render_widget(Clear, popup_area);
    f.render_widget(block.clone(), popup_area);

    let inner = popup_area.inner(&Margin {
        horizontal: 2,
        vertical: 1,
    });
    let label = match app.editing_field {
        Some(EditingField::Url) => "URL:",
        Some(EditingField::Body) => "Body:",
        Some(EditingField::ParamKey(_)) => "Param Key:",
        Some(EditingField::ParamValue(_)) => "Param Value:",
        Some(EditingField::HeaderKey(_)) => "Header Key:",
        Some(EditingField::HeaderValue(_)) => "Header Value:",
        Some(EditingField::AuthToken) => "Bearer Token:",
        Some(EditingField::AuthUsername) => "Username:",
        Some(EditingField::AuthPassword) => "Password:",
        _ => "",
    };
    let before = &app.edit_buffer[..app.cursor_pos.min(app.edit_buffer.len())];
    let after = &app.edit_buffer[app.cursor_pos.min(app.edit_buffer.len())..];
    let text = format!("{}\n{}▌{}", label, before, after);
    let paragraph = Paragraph::new(text).style(Style::default().fg(TEXT).bg(BG));
    f.render_widget(paragraph, inner);
}

// ─── Dialog ──────────────────────────────────────────────────────────
fn draw_dialog(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = if app.dialog_type == DialogType::DeleteConfirm {
        centered_fixed(32, 7, area)
    } else if app.dialog_type == DialogType::History {
        centered_rect(80, 70, area) // History dialog is larger
    } else {
        centered_rect(60, 25, area)
    };
    let title = match app.dialog_type {
        DialogType::ExportCollection => " Export Collection ",
        DialogType::NewCollection => " New Collection ",
        DialogType::DeleteConfirm => "",
        DialogType::RequestName => " Request Name ",
        DialogType::History => " 历史记录 (History) ",
        DialogType::HistorySearch => " 搜索历史 ",
        DialogType::None => return,
    };
    let block = Block::default()
        .title(Span::styled(
            title,
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(BG));
    f.render_widget(Clear, popup_area);
    f.render_widget(block.clone(), popup_area);

    let inner = popup_area.inner(&Margin {
        horizontal: 2,
        vertical: 1,
    });

    if app.dialog_type == DialogType::DeleteConfirm {
        // Compact delete confirm: title + name + narrow Yes/No buttons
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
            ])
            .split(inner);

        let title_para = Paragraph::new("Delete")
            .style(
                Style::default()
                    .fg(ACCENT)
                    .bg(BG)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center);
        f.render_widget(title_para, rows[0]);

        let name_text = if app.dialog_message.is_empty() {
            "(unnamed)".to_string()
        } else {
            format!("\"{}\"", app.dialog_message)
        };
        let name = Paragraph::new(name_text)
            .style(
                Style::default()
                    .fg(TEXT)
                    .bg(BG)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center);
        f.render_widget(name, rows[1]);

        let btn_area = rows[2];

        let yes_style = if app.dialog_option_selected {
            Style::default()
                .bg(BORDER_ACTIVE)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White).bg(SURFACE)
        };
        let no_style = if !app.dialog_option_selected {
            Style::default()
                .bg(BORDER_ACTIVE)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White).bg(SURFACE)
        };

        let btn_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(8),
                Constraint::Length(4),
                Constraint::Length(8),
                Constraint::Min(0),
            ])
            .split(btn_area);

        let yes_btn = Paragraph::new(" Yes ")
            .alignment(Alignment::Center)
            .style(yes_style);
        let no_btn = Paragraph::new(" No ")
            .alignment(Alignment::Center)
            .style(no_style);

        f.render_widget(yes_btn, btn_layout[1]);
        f.render_widget(no_btn, btn_layout[3]);
        return;
    }

    // History dialog
    if app.dialog_type == DialogType::History {
        let entries = app.history_manager.get_all_entries();
        if entries.is_empty() {
            let text = Paragraph::new("No history entries yet.\n\nPress Enter or Esc to close.")
                .style(Style::default().fg(TEXT).bg(BG))
                .alignment(Alignment::Center);
            f.render_widget(text, inner);
        } else {
            // Create a table-like view for history entries
            let rows: Vec<_> = entries
                .iter()
                .enumerate()
                .map(|(idx, entry)| {
                    let is_selected = idx == app.history_selected;
                    let method_color = match entry.request.method {
                        HttpMethod::GET => Color::Rgb(100, 180, 255),
                        HttpMethod::POST => Color::Rgb(100, 255, 150),
                        HttpMethod::PUT => Color::Rgb(255, 200, 100),
                        HttpMethod::DELETE => Color::Rgb(255, 100, 100),
                        HttpMethod::PATCH => Color::Rgb(200, 100, 255),
                        _ => TEXT,
                    };
                    let status_str = entry
                        .response_status
                        .map(|s| format!("{}", s))
                        .unwrap_or_else(|| "-".to_string());
                    let status_color = entry.response_status.map_or(TEXT_DIM, |s| {
                        if s >= 200 && s < 300 {
                            Color::Rgb(100, 255, 150)
                        } else if s >= 400 {
                            Color::Rgb(255, 100, 100)
                        } else {
                            Color::Rgb(255, 200, 100)
                        }
                    });
                    let time_str = format_timestamp(entry.timestamp);
                    let duration_str = entry
                        .duration_ms
                        .map(|d| format!("{}ms", d))
                        .unwrap_or_else(|| "-".to_string());

                    let style = if is_selected {
                        Style::default()
                            .bg(BORDER_ACTIVE)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(TEXT)
                    };

                    let line = Line::from(vec![
                        Span::raw(format!("{:2}  ", idx + 1)),
                        Span::styled(
                            format!("{:6} ", entry.request.method),
                            style.fg(method_color),
                        ),
                        Span::raw(crate::utils::truncate(&entry.request.url, 40)),
                        Span::raw("  "),
                        Span::styled(status_str, style.fg(status_color)),
                        Span::raw("  "),
                        Span::styled(duration_str, style.fg(TEXT_DIM)),
                        Span::raw("  "),
                        Span::styled(time_str, style.fg(TEXT_DIM)),
                    ]);
                    ListItem::new(line).style(style)
                })
                .collect();

            let list = List::new(rows).block(
                Block::default()
                    .borders(Borders::NONE)
                    .style(Style::default().bg(BG)),
            );
            f.render_widget(list, inner);
        }
        return;
    }

    // Text-input dialogs
    let before = &app.dialog_buffer[..app.dialog_cursor.min(app.dialog_buffer.len())];
    let after = &app.dialog_buffer[app.dialog_cursor.min(app.dialog_buffer.len())..];
    let text = format!("{}\n\n{}▌{}", app.dialog_message, before, after);
    let paragraph = Paragraph::new(text).style(Style::default().fg(TEXT).bg(BG));
    f.render_widget(paragraph, inner);
}

// ─── Loading ─────────────────────────────────────────────────────────
fn draw_loading(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(24, 12, area);
    let block = Block::default()
        .title(Span::styled(
            " Loading ",
            Style::default().fg(WARN).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(WARN))
        .style(Style::default().bg(BG));
    f.render_widget(Clear, popup_area);
    f.render_widget(block, popup_area);

    let inner = popup_area.inner(&Margin {
        horizontal: 1,
        vertical: 1,
    });
    let spinner = Paragraph::new("Sending request...")
        .alignment(Alignment::Center)
        .style(Style::default().fg(WARN).add_modifier(Modifier::BOLD));
    f.render_widget(spinner, inner);
}

// ─── Helpers ─────────────────────────────────────────────────────────
fn centered_fixed(width: u16, height: u16, r: Rect) -> Rect {
    let w = width.min(r.width);
    let h = height.min(r.height);
    Rect {
        x: r.x + (r.width.saturating_sub(w)) / 2,
        y: r.y + (r.height.saturating_sub(h)) / 2,
        width: w,
        height: h,
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn format_timestamp(timestamp: DateTime<Local>) -> String {
    use chrono::Local;

    let now = Local::now();
    let diff = now.signed_duration_since(timestamp);
    let diff_secs = diff.num_seconds();

    if diff_secs < 60 {
        "just now".to_string()
    } else if diff_secs < 3600 {
        format!("{}m ago", diff_secs / 60)
    } else if diff_secs < 86400 {
        format!("{}h ago", diff_secs / 3600)
    } else {
        format!("{}d ago", diff_secs / 86400)
    }
}
