pub mod app;
pub mod events;
pub mod shortcuts;
pub mod ui;

use crate::tui::app::App;
use anyhow::Result;
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

pub async fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new()?;

    while app.running {
        if app.pending_send {
            app.pending_send = false;
            app.loading = true;
            terminal.draw(|f| ui::draw(f, &mut app))?;

            let req = app.build_request_with_env(&app.current_request);

            match crate::http_client::send_request(&req, None).await {
                Ok(resp) => {
                    app.add_to_history(req, resp.clone());
                    app.response = Some(resp);
                    app.response_scroll = (0, 0);
                    app.set_status("Request sent successfully");
                }
                Err(e) => {
                    app.response_scroll = (0, 0);
                    app.set_status(format!("Request failed: {}", e));
                }
            }
            app.loading = false;
        }

        terminal.draw(|f| ui::draw(f, &mut app))?;

        if events::handle_events(&mut app)? {
            app.running = false;
        }

        app.tick();
    }

    disable_raw_mode()?;
    let stdout = terminal.backend_mut();
    stdout.execute(LeaveAlternateScreen)?;
    Ok(())
}
