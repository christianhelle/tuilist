//! What the event loop does with what the terminal sends it. The terminal
//! itself stays with the caller, so this runs in tests.

use std::io::Write;

use anyhow::Result;
use ratatui::{
    crossterm::event::{Event as TermEvent, KeyEventKind, MouseButton, MouseEventKind},
    layout::Rect,
};

use crate::tui::{
    app::{AppState, Effect},
    clipboard,
    keymap::Keymap,
    taskboxes::TaskBoxes,
    ui::{self, Mouse},
};

pub fn on_terminal_event(
    app: &mut AppState,
    boxes: &mut TaskBoxes,
    keymap: &mut Keymap,
    area: Rect,
    event: TermEvent,
) -> Result<()> {
    app.page = ui::page_height(area, app);
    let effect = match event {
        TermEvent::Key(key) if key.kind == KeyEventKind::Press => app.on_key(key, keymap),
        TermEvent::Mouse(mouse) => {
            let action = match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => Mouse::Click,
                MouseEventKind::ScrollUp => Mouse::ScrollUp,
                MouseEventKind::ScrollDown => Mouse::ScrollDown,
                _ => return Ok(()),
            };
            let hit = ui::hit(area, app, mouse.column, mouse.row);
            app.on_mouse(action, hit)
        }
        _ => return Ok(()),
    };
    if let Effect::Copy(text) = &effect {
        let mut out = std::io::stdout();
        out.write_all(clipboard::osc52(text).as_bytes())?;
        out.flush()?;
    }
    boxes.apply(app, effect);
    Ok(())
}
