use std::{
    io,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
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
    accounts::{Account, Accounts, Registry},
    auth::{
        DeviceCode, KeyringStore, MemoryStore, OAuthClient, OAuthConfig, SecretStore, TokenProvider,
    },
    core::{fake::FakeBackend, run_engine, sync_lists, sync_tasks},
    graph::{GRAPH_V1, GraphClient},
    store::Store,
    tui::{
        app::AppState, demo, keymap::Keymap, run_loop::on_terminal_event, session::Session,
        taskboxes::TaskBoxes, ui,
    },
};

/// Delegated Graph scopes for reading and writing tasks.
const SCOPES: [&str; 3] = ["offline_access", "User.Read", "Tasks.ReadWrite"];

/// The Entra ID app registration tuilook and tuilist share (a public client;
/// not a secret).
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
    #[command(subcommand)]
    command: Option<CliCommand>,
}

#[derive(Subcommand)]
enum CliCommand {
    /// Sign in with a device code. Run it again to add another account.
    Login,
    /// Forget an account's sign-in and cached tasks.
    Logout {
        /// The account's address, or the start of it. Needed when signed in
        /// to several accounts.
        account: Option<String>,
    },
    /// List the signed-in accounts.
    Accounts,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let runtime = tokio::runtime::Runtime::new()?;
    match &cli.command {
        Some(CliCommand::Login) => runtime.block_on(login(&cli)),
        Some(CliCommand::Logout { account }) => runtime.block_on(logout(account.as_deref())),
        Some(CliCommand::Accounts) => {
            let accounts = signed_in()?;
            if accounts.0.is_empty() {
                println!("Not signed in.");
            }
            for account in accounts.0 {
                println!("{}", account.address);
            }
            Ok(())
        }
        None if cli.demo => run_demo(&runtime),
        None => bail!("Not signed in. Run todo login, or todo --demo to look around."),
    }
}

fn run_demo(runtime: &tokio::runtime::Runtime) -> Result<()> {
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

fn client_id(cli: &Cli) -> Result<String> {
    let client_id = cli.client_id.clone();
    if client_id.is_empty() {
        bail!(
            "no client ID: register a public client app in Entra ID, then pass \
             --client-id or set TUILIST_CLIENT_ID"
        );
    }
    Ok(client_id)
}

fn oauth(tenant: &str, client_id: &str) -> OAuthClient {
    OAuthClient::new(OAuthConfig {
        authority: "https://login.microsoftonline.com".to_owned(),
        tenant: tenant.to_owned(),
        client_id: client_id.to_owned(),
        scopes: SCOPES.map(str::to_owned).to_vec(),
    })
}

/// Signs in from the command line.
async fn login(cli: &Cli) -> Result<()> {
    let address = sign_in(&cli.tenant, &client_id(cli)?, |code| {
        println!("{}", code.message)
    })
    .await?;
    println!("Signed in as {address}.");
    Ok(())
}

/// Signs in with a device code shown by `show_code`, keeping the refresh
/// token in memory until Graph says whose it is, then saves the account
/// and returns its address.
async fn sign_in(
    tenant: &str,
    client_id: &str,
    show_code: impl FnOnce(&DeviceCode),
) -> Result<String> {
    let memory = Arc::new(MemoryStore::default());
    let tokens = Arc::new(TokenProvider::new(oauth(tenant, client_id), memory.clone()));
    tokens.login(show_code).await?;
    let address = GraphClient::new(GRAPH_V1, tokens)
        .address()
        .await?
        .context("Microsoft did not say which account signed in")?;
    let refresh_token = memory
        .load()?
        .context("Microsoft did not let tuilist stay signed in (no refresh token)")?;
    KeyringStore::new(&address)?.save(&refresh_token)?;
    registry()?.change(|accounts| {
        accounts.upsert(Account {
            address: address.clone(),
            tenant: tenant.to_owned(),
            client_id: client_id.to_owned(),
        })
    })?;
    Ok(address)
}

async fn logout(query: Option<&str>) -> Result<()> {
    let accounts = signed_in()?;
    let account = match (query, &accounts.0[..]) {
        (Some(query), _) => accounts.find(query)?.clone(),
        (None, [only]) => only.clone(),
        (None, []) => {
            println!("Not signed in.");
            return Ok(());
        }
        (None, _) => {
            bail!("signed in to several accounts; say which: `todo logout <address>`")
        }
    };
    forget(&account.address)?;
    println!("Signed out of {}.", account.address);
    Ok(())
}

/// Forgets the account at `address`: its sign-in, its place in the list and
/// its cached tasks. Its cache must be closed.
fn forget(address: &str) -> Result<()> {
    KeyringStore::new(address)?.clear()?;
    Ok(registry()?.forget(address)?)
}

fn signed_in() -> Result<Accounts> {
    Ok(registry()?.load()?)
}

fn data_dir() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "tuilist")
        .context("no home directory to keep the task cache in")?;
    std::fs::create_dir_all(dirs.data_dir())?;
    Ok(dirs.data_dir().to_owned())
}

/// The accounts signed in to, and where each one's tasks are cached.
fn registry() -> Result<Registry> {
    Ok(Registry::at(data_dir()?))
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
