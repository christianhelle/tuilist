use std::{
    io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::Result;
use clap::Parser;
use jiff::Zoned;
use ratatui::{
    DefaultTerminal,
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture},
        execute,
    },
    layout::Rect,
};
use tokio::sync::mpsc;
use tuilist::{
    core::{fake::FakeBackend, run_engine, sync_lists, sync_tasks},
    store::Store,
    tui::{
        app::AppState, demo, keymap::Keymap, run_loop::on_terminal_event, session::Session,
        taskboxes::TaskBoxes, ui,
    },
};

/// tuilist's own Entra ID app registration (a public client; not a secret).
const DEFAULT_CLIENT_ID: &str = "5de297ab-80a5-4cfc-bdea-38e33b917993";

#[derive(Parser)]
#[command(
    name = "todo",
    version,
    about = "A terminal client for Microsoft To Do"
)]
struct Cli {
    /// Entra ID application (client) ID to sign in with; use your own if your
    /// tenant blocks tuilist's.
    #[arg(long, env = "TUILIST_CLIENT_ID", default_value = DEFAULT_CLIENT_ID, global = true)]
    client_id: String,
    /// Tenant to sign in to: consumers (personal accounts), organizations,
    /// common or a tenant ID.
    #[arg(
        long,
        env = "TUILIST_TENANT",
        default_value = "consumers",
        global = true
    )]
    tenant: String,
    /// Browse built-in sample lists instead of signing in.
    #[arg(long)]
    demo: bool,
    /// Seconds between syncs; 0 syncs only at start and on F5.
    #[arg(long, env = "TUILIST_SYNC_EVERY", default_value_t = 60, global = true)]
    sync_every: u64,
    /// Don't ask GitHub for a newer tuilist at start.
    #[arg(
        long,
        env = "TUILIST_NO_UPDATE_CHECK",
        value_parser = clap::builder::FalseyValueParser::new()
    )]
    no_update_check: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if !cli.demo {
        anyhow::bail!("Not signed in. Run todo login, or todo --demo to look around.")
    }
    let runtime = tokio::runtime::Runtime::new()?;
    let dir = tempfile::tempdir()?;
    let seeded = runtime.block_on(seed_demo(dir.path()))?;
    let mut accounts = Vec::new();
    let mut engines = Vec::new();
    for (address, store, backend, path) in seeded {
        let display = Store::open(&path)?;
        let (commands, receiver) = mpsc::unbounded_channel();
        let (events, receiver_events) = mpsc::channel(64);
        runtime.spawn(run_engine(backend, store, receiver, events));
        accounts.push((address, display));
        engines.push((commands, receiver_events));
    }
    let mut app = AppState {
        clock: Some(Zoned::now()),
        ..AppState::default()
    };
    let mut boxes = TaskBoxes::new(accounts);
    for (account, (commands, events)) in engines.into_iter().enumerate() {
        boxes.attach(account, commands, events);
    }
    boxes.load(&mut app)?;
    if let Some(session) = Session::load() {
        session.apply(&mut app);
        boxes.load(&mut app)?;
    }
    with_terminal(|terminal| drive(terminal, &mut app, &mut boxes))?;
    Session::of(&app).save()
}

async fn seed_demo(dir: &Path) -> Result<Vec<(String, Store, FakeBackend, PathBuf)>> {
    let mut accounts = Vec::new();
    for (index, (address, backend)) in demo::accounts().into_iter().enumerate() {
        let path = dir.join(format!("account-{index}.db"));
        let mut store = Store::open(&path)?;
        sync_lists(&backend, &mut store).await?;
        let lists = store.lists()?;
        for list in &lists {
            sync_tasks(&backend, &mut store, &list.id).await?;
        }
        for list in &lists {
            for task in store.tasks_in(&list.id)? {
                let steps = backend.steps_of(&task.id);
                if !steps.is_empty() {
                    store.put_steps(&task.id, &steps)?;
                }
            }
        }
        accounts.push((address, store, backend, path));
    }
    Ok(accounts)
}

fn with_terminal<T>(f: impl FnOnce(&mut DefaultTerminal) -> Result<T>) -> Result<T> {
    let mut terminal = ratatui::init();
    let result = execute!(io::stdout(), EnableMouseCapture)
        .map_err(anyhow::Error::from)
        .and_then(|()| f(&mut terminal));
    let _ = execute!(io::stdout(), DisableMouseCapture);
    ratatui::restore();
    result
}

fn drive(terminal: &mut DefaultTerminal, app: &mut AppState, boxes: &mut TaskBoxes) -> Result<()> {
    let mut keymap = Keymap::default();
    while !app.should_quit {
        boxes.poll(app);
        app.expire_status(Instant::now());
        terminal.draw(|frame| ui::draw(frame, app))?;
        if event::poll(Duration::from_millis(100))? {
            let size = terminal.size()?;
            let area = Rect::new(0, 0, size.width, size.height);
            on_terminal_event(app, boxes, &mut keymap, area, event::read()?)?;
            app.clock = Some(Zoned::now());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signs_in_with_the_tuilist_app_registration_by_default() {
        let cli = Cli::try_parse_from(["todo"]).unwrap();

        assert_eq!(
            cli.client_id.as_str(),
            "5de297ab-80a5-4cfc-bdea-38e33b917993"
        );
    }

    #[test]
    fn signs_in_to_personal_accounts_by_default() {
        let cli = Cli::try_parse_from(["todo"]).unwrap();

        assert_eq!(cli.tenant, "consumers");
    }

    #[test]
    fn the_client_id_can_be_overridden() {
        let cli = Cli::try_parse_from(["todo", "--client-id", "my-own-app"]).unwrap();

        assert_eq!(cli.client_id, "my-own-app");
    }

    #[test]
    fn syncs_every_minute_unless_told_otherwise() {
        let cli = Cli::try_parse_from(["todo"]).unwrap();
        assert_eq!(cli.sync_every, 60);

        let cli = Cli::try_parse_from(["todo", "--sync-every", "0"]).unwrap();
        assert_eq!(cli.sync_every, 0);
    }

    #[test]
    fn the_update_check_can_be_turned_off() {
        let cli = Cli::try_parse_from(["todo", "--no-update-check"]).unwrap();

        assert!(cli.no_update_check);
    }
}
