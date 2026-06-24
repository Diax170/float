mod cell;
mod compositor;
mod config;
mod escape;
mod frame;
mod input;
mod pty;
mod window;
mod wm;

use std::io;
use std::time::Duration;

use crossterm::{
    cursor, event, execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen, enable_raw_mode},
};

use wm::WindowManager;

fn main() -> anyhow::Result<()> {
    let config = config::load();
    let (cols, rows) = terminal::size()?;
    let mut wm = WindowManager::new(config, cols, rows)?;
    let mut stdout = io::stdout();

    enable_raw_mode()?;
    execute!(
        stdout,
        EnterAlternateScreen,
        cursor::Hide,
        event::EnableMouseCapture,
    )?;

    let restore = || {
        let mut out = io::stdout();
        let _ = execute!(
            out,
            cursor::Show,
            LeaveAlternateScreen,
            event::DisableMouseCapture
        );
        let _ = terminal::disable_raw_mode();
    };

    let result = run_event_loop(&mut wm);

    restore();
    result
}

fn run_event_loop(wm: &mut WindowManager) -> anyhow::Result<()> {
    loop {
        // Flush stale Esc if the timeout elapsed without a follow-up key
        wm.expire_esc();

        wm.process_all();
        if wm.is_quit_requested() {
            return Ok(());
        }

        if wm.is_dirty() {
            wm.composite()?;
            wm.clear_dirty();
        }

        wm.reap_dead_windows()?;

        if event::poll(Duration::from_millis(wm.config.poll_interval_ms))? {
            match event::read()? {
                event::Event::Key(key) => wm.handle_key(key)?,
                event::Event::Mouse(mouse) => wm.handle_mouse(mouse)?,
                event::Event::Resize(new_cols, new_rows) => {
                    wm.resize_screen(new_rows, new_cols);
                }
                _ => {}
            }
        }
    }
}
