//! The keys help, and the hints shown on the status line.

use crate::tui::app::{AppState, Pane};

pub struct Row {
    pub key: &'static str,
    pub what: &'static str,
}

const fn row(key: &'static str, what: &'static str) -> Row {
    Row { key, what }
}

/// The keys for the screen being shown.
pub fn sections(app: &AppState) -> Vec<(&'static str, Vec<Row>)> {
    let mut sections = vec![
        (
            "Every pane",
            vec![
                row("Tab  Shift-Tab", "next or previous pane"),
                row("j k  ↓ ↑", "move"),
                row("l h  → ←", "unfold, fold or go back"),
                row("gg G  Home End", "first or last"),
                row("PgDn PgUp", "a page"),
                row("Ctrl-D Ctrl-U", "half a page"),
                row("Enter", "open, or fold a group"),
                row("z", "zoom the pane"),
                row("b", "hide the lists"),
                row("L", "details below the tasks"),
                row("F5  Ctrl-R", "sync now"),
                row("?", "these keys"),
                row("q  Ctrl-C", "quit"),
            ],
        ),
        (
            "Lists",
            vec![
                row("a", "new list"),
                row("F2", "rename the list"),
                row("d  D", "delete the list and its tasks"),
            ],
        ),
        (
            "Tasks",
            vec![
                row("a", "add a task"),
                row("x", "complete or reopen"),
                row("*", "star or unstar"),
                row("e  i", "edit the task"),
                row("F2", "rename the task"),
                row("t", "due today, or clear it"),
                row("T", "ask for a due date"),
                row("s", "add a step"),
                row("m", "move to another list"),
                row("d  D", "delete"),
                row("y", "copy the title"),
                row("c", "hide or show completed"),
                row("Space", "mark and move down"),
                row("Ctrl-A", "mark every task shown"),
                row("Esc", "clear the marks"),
            ],
        ),
        (
            "Details",
            vec![
                row("h j k l  w b e", "move the cursor"),
                row("v  V  y", "pick and copy"),
                row("x", "check the step, or complete the task"),
                row("s", "add a step"),
                row("i", "edit the task"),
                row("→ ←", "next or previous task"),
            ],
        ),
    ];
    if app.searching() {
        sections.push((
            "Search",
            vec![
                row("Enter", "move into the results"),
                row("Esc", "return to the list"),
            ],
        ));
    }
    sections
}

/// The keys named on the status line for `pane`.
pub fn hint(pane: Pane) -> &'static str {
    match pane {
        Pane::Lists => {
            "  ? help · Tab switch pane · Enter open · a new list · F2 rename · d delete · q quit "
        }
        Pane::Tasks => {
            "  ? help · Tab switch pane · a add · x complete · e edit · / search · d delete · q quit "
        }
        Pane::Details => {
            "  ? help · x complete · s add step · i edit · v select · y copy · q quit "
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::keymap::Keymap;
    use ratatui::crossterm::event::{KeyCode, KeyEvent};

    #[test]
    fn every_key_in_the_shared_and_task_help_is_bound() {
        let app = AppState::default();
        let mut keymap = Keymap::default();
        for (title, rows) in sections(&app) {
            if title == "Details" {
                continue;
            }
            for row in rows {
                for key in row.key.split_whitespace() {
                    if key == "gg" {
                        let mut event = KeyEvent::from(KeyCode::Char('g'));
                        assert_eq!(keymap.action_for(event), None);
                        event = KeyEvent::from(KeyCode::Char('g'));
                        assert_eq!(keymap.action_for(event), Some(crate::model::Action::Top));
                        continue;
                    }
                    let code = match key {
                        "Tab" => KeyCode::Tab,
                        "Shift-Tab" => KeyCode::BackTab,
                        "↓" => KeyCode::Down,
                        "↑" => KeyCode::Up,
                        "→" => KeyCode::Right,
                        "←" => KeyCode::Left,
                        "PgDn" => KeyCode::PageDown,
                        "PgUp" => KeyCode::PageUp,
                        "Ctrl-D" => KeyCode::Char('d'),
                        "Ctrl-U" => KeyCode::Char('u'),
                        "Ctrl-R" => KeyCode::Char('r'),
                        "Ctrl-C" => KeyCode::Char('c'),
                        "Ctrl-A" => KeyCode::Char('a'),
                        "Home" => KeyCode::Home,
                        "End" => KeyCode::End,
                        "Enter" => KeyCode::Enter,
                        "Esc" => KeyCode::Esc,
                        "F2" => KeyCode::F(2),
                        "F5" => KeyCode::F(5),
                        "Space" => KeyCode::Char(' '),
                        key if key.chars().count() == 1 => {
                            KeyCode::Char(key.chars().next().unwrap())
                        }
                        key => panic!("the test does not know {key}"),
                    };
                    let mut event = KeyEvent::from(code);
                    if key.starts_with("Ctrl-") {
                        event =
                            KeyEvent::new(code, ratatui::crossterm::event::KeyModifiers::CONTROL);
                    }
                    let action = keymap.action_for(event);
                    assert!(
                        action.is_some() || key == "gg" || key == "G",
                        "{title}: {key} has no binding"
                    );
                }
            }
        }
    }

    #[test]
    fn the_hints_name_keys_that_do_something() {
        assert!(hint(Pane::Lists).contains("F2"));
        assert!(hint(Pane::Tasks).contains("/ search"));
        assert!(hint(Pane::Details).contains("y copy"));
    }
}
