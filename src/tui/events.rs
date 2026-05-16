use crate::tui::app::{App, DialogType, EditingField, InputMode};
use crate::tui::shortcuts::{self, Action};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use std::time::Duration;

pub fn handle_events(app: &mut App) -> anyhow::Result<bool> {
    if event::poll(Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                return Ok(false);
            }
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(true);
            }

            match app.input_mode {
                InputMode::Normal => handle_normal(app, key),
                InputMode::Editing => {
                    if app.dialog_type != DialogType::None {
                        handle_dialog(app, key);
                    } else {
                        handle_editing(app, key);
                    }
                }
            }
        }
    }
    Ok(false)
}

fn handle_normal(app: &mut App, key: event::KeyEvent) {
    let (action, new_prefix) = shortcuts::parse(
        key,
        app.active_pane,
        app.sidebar_tab,
        app.request_tab,
        app.pending_window_prefix,
    );
    app.pending_window_prefix = new_prefix;

    if action == Action::None && app.pending_window_prefix {
        return;
    }

    match action {
        // Window switching
        Action::SwitchPane(1) => app.active_pane = crate::tui::app::ActivePane::Sidebar,
        Action::SwitchPane(2) => app.active_pane = crate::tui::app::ActivePane::UrlBar,
        Action::SwitchPane(3) => app.active_pane = crate::tui::app::ActivePane::RequestTabs,
        Action::SwitchPane(4) => app.active_pane = crate::tui::app::ActivePane::Response,
        Action::NextPane => app.move_focus_next(),
        Action::PrevPane => app.move_focus_prev(),
        Action::WindowPrefix => app.activate_window_prefix(),

        // Global
        Action::Quit => app.running = false,
        Action::SaveRequest => app.save_current_request_to_selected_collection(),
        Action::ExportCollection => {
            app.open_dialog(
                DialogType::ExportCollection,
                "Export format [json/postman]:",
            );
        }
        Action::NewCollection => {
            app.open_dialog(DialogType::NewCollection, "New collection name:");
        }
        Action::NewRequest => app.new_request(),
        Action::ShowHelp => {
            app.set_status("Shortcuts: Tab=focus | x=prefix | u=url | Enter=send | n=name | d=del | e=edit | v=edit value | i=new req | f=new coll | Ctrl+S=save | Ctrl+E=export | Ctrl+R=history | ?=help");
        }
        Action::ShowHistory => {
            if app.history_manager.get_all_entries().is_empty() {
                app.set_status("No history entries yet. Send some requests first!");
            } else {
                app.open_dialog(
                    DialogType::History,
                    "历史记录 (j/k 选择, Enter 加载, Esc 取消):",
                );
                app.history_selected = 0;
            }
        }
        Action::ShowBookmarks => {
            app.set_status("Bookmarks feature coming soon!");
        }

        // Sidebar
        Action::SidebarNext => {
            app.next_sidebar_item();
            app.try_load_selected_collection_request();
        }
        Action::SidebarPrev => {
            app.prev_sidebar_item();
            app.try_load_selected_collection_request();
        }
        Action::SidebarExpand => {
            if app.sidebar_tab == crate::tui::app::SidebarTab::Collections {
                if let Some(ci) = app.get_selected_collection_index() {
                    let col = &app.data.collections[ci];
                    if !app.collection_expanded.contains(&col.id) {
                        app.collection_expanded.push(col.id.clone());
                    }
                }
            }
        }
        Action::SidebarCollapse => {
            if app.sidebar_tab == crate::tui::app::SidebarTab::Collections {
                if let Some(ci) = app.get_selected_collection_index() {
                    let col = &app.data.collections[ci];
                    app.collection_expanded.retain(|id| id != &col.id);
                }
            }
        }
        Action::SidebarLoad => match app.sidebar_tab {
            crate::tui::app::SidebarTab::Collections => {
                if app.try_load_selected_collection_request() {
                    app.active_pane = crate::tui::app::ActivePane::UrlBar;
                }
            }
            crate::tui::app::SidebarTab::Environments => {
                app.activate_environment(app.sidebar_selected);
            }
        },
        Action::SidebarDelete => app.delete_selected_item(),
        Action::SidebarTabCollections => app.sidebar_tab = crate::tui::app::SidebarTab::Collections,
        Action::SidebarTabEnvironments => {
            app.sidebar_tab = crate::tui::app::SidebarTab::Environments
        }
        Action::CycleTagFilter => app.cycle_tag_filter(),

        // URL Bar
        Action::CycleMethodNext => app.cycle_method(),
        Action::CycleMethodPrev => app.cycle_method_prev(),
        Action::EditUrl => app.start_editing(EditingField::Url),
        Action::EditRequestName => app.start_editing(EditingField::RequestName),
        Action::SendRequest => app.pending_send = true,

        // Request Tabs
        Action::SwitchTabParams => app.request_tab = crate::tui::app::RequestTab::Params,
        Action::SwitchTabHeaders => app.request_tab = crate::tui::app::RequestTab::Headers,
        Action::SwitchTabBody => app.request_tab = crate::tui::app::RequestTab::Body,
        Action::SwitchTabAuth => app.request_tab = crate::tui::app::RequestTab::Auth,
        Action::TabNext => {
            app.request_tab = match app.request_tab {
                crate::tui::app::RequestTab::Params => crate::tui::app::RequestTab::Headers,
                crate::tui::app::RequestTab::Headers => crate::tui::app::RequestTab::Body,
                crate::tui::app::RequestTab::Body => crate::tui::app::RequestTab::Auth,
                crate::tui::app::RequestTab::Auth => crate::tui::app::RequestTab::Params,
            };
        }
        Action::TabPrev => {
            app.request_tab = match app.request_tab {
                crate::tui::app::RequestTab::Params => crate::tui::app::RequestTab::Auth,
                crate::tui::app::RequestTab::Headers => crate::tui::app::RequestTab::Params,
                crate::tui::app::RequestTab::Body => crate::tui::app::RequestTab::Headers,
                crate::tui::app::RequestTab::Auth => crate::tui::app::RequestTab::Body,
            };
        }
        Action::AddRow => match app.request_tab {
            crate::tui::app::RequestTab::Params => app.add_param(),
            crate::tui::app::RequestTab::Headers => app.add_header(),
            _ => {}
        },
        Action::DeleteRow => match app.request_tab {
            crate::tui::app::RequestTab::Params => {
                if let Some(i) = app.param_list_state.selected() {
                    app.remove_param(i);
                }
            }
            crate::tui::app::RequestTab::Headers => {
                if let Some(i) = app.header_list_state.selected() {
                    app.remove_header(i);
                }
            }
            _ => {}
        },
        Action::ToggleEnabled => match app.request_tab {
            crate::tui::app::RequestTab::Params => {
                if let Some(i) = app.param_list_state.selected() {
                    app.toggle_param(i);
                }
            }
            crate::tui::app::RequestTab::Headers => {
                if let Some(i) = app.header_list_state.selected() {
                    app.toggle_header(i);
                }
            }
            _ => {}
        },
        Action::EditKey => match app.request_tab {
            crate::tui::app::RequestTab::Params => {
                if let Some(i) = app.param_list_state.selected() {
                    app.start_editing(EditingField::ParamKey(i));
                }
            }
            crate::tui::app::RequestTab::Headers => {
                if let Some(i) = app.header_list_state.selected() {
                    app.cycle_header_key(i);
                }
            }
            crate::tui::app::RequestTab::Body => app.start_editing(EditingField::Body),
            crate::tui::app::RequestTab::Auth => app.start_editing(EditingField::AuthToken),
        },
        Action::EditValue => match app.request_tab {
            crate::tui::app::RequestTab::Params => {
                if let Some(i) = app.param_list_state.selected() {
                    app.start_editing(EditingField::ParamValue(i));
                }
            }
            crate::tui::app::RequestTab::Headers => {
                if let Some(i) = app.header_list_state.selected() {
                    app.cycle_header_value(i);
                }
            }
            _ => {}
        },
        Action::CycleBodyType => app.cycle_body_type(),
        Action::CycleAuthType => app.cycle_auth_type(),

        // Response
        Action::ScrollUp => app.response_scroll.1 = app.response_scroll.1.saturating_sub(1),
        Action::ScrollDown => app.response_scroll.1 = app.response_scroll.1.saturating_add(1),
        Action::PageUp => app.response_scroll.1 = app.response_scroll.1.saturating_sub(10),
        Action::PageDown => app.response_scroll.1 = app.response_scroll.1.saturating_add(10),
        Action::ScrollHalfPageUp => {
            app.response_scroll.1 = app.response_scroll.1.saturating_sub(20)
        }
        Action::ScrollHalfPageDown => {
            app.response_scroll.1 = app.response_scroll.1.saturating_add(20)
        }
        Action::ScrollTop => app.response_scroll.1 = 0,
        Action::ScrollBottom => app.response_scroll.1 = u16::MAX,
        Action::CopyBody => {
            if let Some(resp) = &app.response {
                let text = match app.response_tab {
                    crate::tui::app::ResponseTab::Body => crate::utils::format_json(&resp.body),
                    crate::tui::app::ResponseTab::Headers => {
                        let mut entries: Vec<_> = resp.headers.iter().collect();
                        entries.sort_by(|a, b| a.0.cmp(b.0));
                        entries
                            .iter()
                            .map(|(k, v)| format!("{}: {}", k, v))
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                };
                match crate::utils::copy_to_clipboard(&text) {
                    Ok(_) => app.set_status(format!("Copied {} bytes to clipboard", text.len())),
                    Err(e) => app.set_status(format!("Copy failed: {}", e)),
                }
            } else {
                app.set_status("No response to copy".to_string());
            }
        }
        Action::ToggleResponseTab => app.cycle_response_tab(),

        Action::None => {}
        _ => {}
    }
}

fn handle_editing(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Enter => app.confirm_edit(),
        KeyCode::Esc => app.cancel_edit(),
        KeyCode::Backspace => app.delete_char(),
        KeyCode::Left => app.move_cursor_left(),
        KeyCode::Right => app.move_cursor_right(),
        KeyCode::Char(c) => app.insert_char(c),
        _ => {}
    }
}

fn handle_dialog(app: &mut App, key: event::KeyEvent) {
    // Choice dialogs (DeleteConfirm, History) use ←/→ to select, Enter to confirm
    if app.dialog_type == DialogType::DeleteConfirm || app.dialog_type == DialogType::History {
        match key.code {
            KeyCode::Enter => {
                if app.dialog_type == DialogType::History {
                    // Load selected history entry into current request
                    let entries = app.history_manager.get_all_entries();
                    if app.history_selected < entries.len() {
                        let entry = &entries[app.history_selected];
                        app.current_request = entry.request.clone();
                        app.set_status(format!(
                            "Loaded history: {} {}",
                            entry.request.method, entry.request.url
                        ));
                    }
                    app.close_dialog();
                    app.history_selected = 0;
                    return;
                }
                if app.dialog_option_selected {
                    app.close_dialog();
                    app.execute_pending_delete();
                } else {
                    app.close_dialog();
                    app.pending_delete = None;
                    app.set_status("Delete cancelled".to_string());
                }
            }
            KeyCode::Esc => {
                app.close_dialog();
                if app.dialog_type == DialogType::History {
                    app.history_selected = 0;
                } else {
                    app.pending_delete = None;
                    app.set_status("Delete cancelled".to_string());
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if app.dialog_type == DialogType::History && app.history_selected > 0 {
                    app.history_selected -= 1;
                } else {
                    app.dialog_option_selected = true;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.dialog_type == DialogType::History {
                    let entries = app.history_manager.get_all_entries();
                    if app.history_selected < entries.len().saturating_sub(1) {
                        app.history_selected += 1;
                    }
                } else {
                    app.dialog_option_selected = false;
                }
            }
            KeyCode::Left => app.dialog_option_selected = true,
            KeyCode::Right => app.dialog_option_selected = false,
            _ => {}
        }
        return;
    }

    // Text-input dialogs
    match key.code {
        KeyCode::Enter => {
            let dtype = app.dialog_type;
            if let Some(value) = app.confirm_dialog() {
                match dtype {
                    DialogType::ExportCollection => {
                        if let Some(content) = app.export_collection(&value) {
                            let filename =
                                format!("{}.json", app.current_request.name.replace(' ', "_"));
                            if let Err(e) = std::fs::write(&filename, &content) {
                                app.set_status(format!("Failed to write file: {}", e));
                            } else {
                                app.set_status(format!("Exported to {}", filename));
                            }
                        }
                    }
                    DialogType::NewCollection => app.create_collection(&value),
                    DialogType::RequestName => {
                        app.current_request.name = value;
                        app.do_save_request();
                    }
                    _ => {}
                }
            }
        }
        KeyCode::Esc => app.close_dialog(),
        KeyCode::Backspace => app.dialog_delete_char(),
        KeyCode::Left => app.dialog_move_cursor_left(),
        KeyCode::Right => app.dialog_move_cursor_right(),
        KeyCode::Char(c) => app.dialog_insert_char(c),
        _ => {}
    }
}
