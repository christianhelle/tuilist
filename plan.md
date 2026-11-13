# TUIList — Implementation Plan

Nov 13, 2026 · @Christian Helle

## 1. Overview

TUIList is a terminal client for Microsoft To Do: three panes (lists, tasks, details), vim keys, an offline SQLite cache and background delta sync through Microsoft Graph. It is built by reusing tuilook's architecture and copying its shared modules, so look, keys and repo conventions match tuilook and tuicalc.

| Item | Value |
| --- | --- |
| Project / repo | `TUIList`, `github.com/christianhelle/tuilist` |
| Crate name | `tuilist` |
| Binary | `todo` (`[[bin]] name = "todo"`) |
| Env var prefix | `TUILIST_` |
| Language / UI | Rust edition 2024, `rust-version = "1.95"`, Ratatui 0.30 |
| License | MIT, `Copyright (c) 2026 Christian Helle` |
| First release | `0.1.0` |

**Goals for 0.1.0**

- Sign in with device code; personal accounts work with no options, work/school with `--tenant` and `--client-id`.
- Browse lists and tasks from cache at once; sync in the background every 60 s and on `F5`.
- Lists: create, rename, delete.
- Tasks: add, rename, complete/reopen, star (importance), due date, reminder, notes, move to another list, delete, multi-mark.
- Steps (checklist items): add, check, rename, delete.
- Search across cached tasks, `?` help, `--demo` mode, session restore, self-update.
- README, CHANGELOG, LICENSE, CI, release and pages workflows, install scripts from the first commit.

**Non-goals for 0.1.0**

- My Day, list groups, manual task order, shared-list management: the Graph To Do API does not expose them.
- Creating or editing recurrence, file attachments, categories (shown read-only where cheap).
- Offline write queue: writes need a connection; a failed write reverts in the UI.
- Non-interactive task commands (`todo add ...`); see open questions.

**Working rules for the implementing model**

- TDD where it makes sense: write the failing test, then the code. Nano commits: one small behaviour per commit, each one passing `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test`.
- No AI tool or model named in commit messages, co-author lines or PR text. PRs are assigned to `christianhelle` with label `enhancement`.
- Every user-visible change adds a line under `## [Unreleased]` in `CHANGELOG.md` in the same commit or PR.
- Clone `christianhelle/tuilook` and `christianhelle/tuicalc` next to the new repo first. When this plan says *copy*, copy the file and rename identifiers; do not rewrite from scratch.

## 2. Reference projects

tuilook is the template for everything network- and account-related; tuicalc confirms the shared conventions. Both were read at their `main` heads on 2026-11-13 (tuilook 0.1.5 + unreleased, tuicalc 0.1.1 + unreleased).

**Copy from tuilook, renaming `tuilook` → `tuilist`, `otlk` → `todo`, `TUILOOK_` → `TUILIST_`**

| tuilook file | Use in TUIList | Changes |
| --- | --- | --- |
| `src/auth/{mod,http,oauth,provider,secrets}.rs` | Same paths | Keyring service and piece prefix renamed; nothing else |
| `src/graph/client.rs` | Same path | Drop `put_chunk` and `get_bytes` (attachment upload/download) |
| `src/accounts.rs` | Same path | Drop `signature` field |
| `src/update.rs` | Same path | Repo, binary and env names |
| `src/tui/vim.rs`, `editor.rs`, `numbers.rs` | Same paths | None; used for the notes editor |
| `src/tui/input.rs`, `filter_list.rs`, `clipboard.rs` | Same paths | None |
| `src/tui/dialog.rs`, `help.rs`, `keymap.rs`, `run_loop.rs`, `screen.rs`, `session.rs`, `account_manager.rs` | Same paths, as starting points | Trim to the dialogs, actions and screens listed in sections 6 and 7 |
| `src/tui/mover.rs`, `folder_edit.rs` | `mover.rs` (move task to list), `list_edit.rs` (name prompt) | Folder → list |
| `tests/support/mod.rs`, `tests/auth_*.rs`, `tests/graph_client.rs`, `tests/keyring_long.rs`, `tests/update.rs` | Same paths | Crate name |
| `.github/workflows/{ci,release,pages}.yml` | Same paths | Names only; see section 9 |
| `docs/install.sh`, `docs/install.ps1` | Same paths | `GITHUB_REPO`, `BINARY_NAME`, URLs |
| `.gitignore`, `.gitattributes`, `.devcontainer/devcontainer.json`, `LICENSE`, `build.rs` | Same paths | `build.rs`: keep the `TUILIST_TARGET` line; keep the icon block only once `assets/icon.ico` exists |

**Conventions both projects share, to keep**

- Layering: `model` (types, no IO) → `store` (SQLite) → `graph` (HTTP) → `core` (engine; owns every store write) → `tui` (state, keys, drawing). `main.rs` wires them and owns the terminal.
- The UI never does IO. A key becomes an `Action`; `AppState` applies it and returns an `Effect`; the caller turns effects into engine `Command`s; the engine answers with `Event`s over a channel.
- A backend trait with one real implementation (Graph) and a `FakeBackend` in `core::fake` that serves both `--demo` and tests.
- Tests: integration tests in `tests/`, named as sentences (`deleting_asks_first`), `wiremock` for HTTP, `insta` snapshots of a `TestBackend` render, an injectable `Sleeper` so retries and polling never wait.
- Doc comments say what a thing is for in plain words; module files open with a `//!` line.
- User-facing text is plain and short: "Move 1 message to Deleted Items?", "y / Enter delete   Esc or any other key cancel".

**Look and feel, taken from `src/tui/ui.rs` in tuilook and `src/ui.rs` in tuicalc**

| Element | Style |
| --- | --- |
| Pane | `Block::bordered()`, title `" {title} "`; `BorderType::Thick` when focused, `Plain` otherwise; no border colour |
| Selected row | ` >  ` marker in the first two columns; other rows indented two spaces |
| Picked row in a dialog list | `Modifier::REVERSED` |
| Keys in help and hints | `Color::Cyan`; the accepting key in a dialog `Color::Yellow`; a destructive accepting key `Color::Red` |
| Dialog border | Yellow for prompts and pickers, Red for destructive confirms, Cyan for neutral choices; drawn over `Clear` |
| Dim text (hints, empty states, rules, counts) | `Color::DarkGray` |
| Field labels in forms | Cyan + bold when focused, DarkGray otherwise |
| Mode badge | Black on Blue `NORMAL`, Green `INSERT`, Magenta `VISUAL` / `V-LINE` |
| Update badge | Black on Yellow `update 0.2.0`, Black on Green `restart for 0.2.0` |
| Errors | Red text in the status line |
| Status line | Last row, no border: badges, then status text or key hints joined by `·`, starting with `? help` |
| Line numbers in editor and details | Hybrid: cursor line absolute in Yellow bold, others relative in DarkGray |
| Group header | `▾ Name (n)` open, `▸ Name (n)` folded |

## 3. Tech stack

Use tuilook's `Cargo.toml` as the base and the versions in its `Cargo.lock`; run `cargo update` only if a build needs it.

```toml
[package]
name = "tuilist"
version = "0.1.0"
edition = "2024"
rust-version = "1.95"
description = "A terminal client for Microsoft To Do, with vim-style keys"
license = "MIT"
readme = "README.md"
repository = "https://github.com/christianhelle/tuilist"
keywords = ["todo", "tasks", "microsoft-to-do", "tui", "terminal"]
categories = ["command-line-utilities"]
exclude = [".github/", ".gitattributes", "images/"]

[[bin]]
name = "todo"
path = "src/main.rs"

[dependencies]
anyhow = "1.0.104"
clap = { version = "4.6.7", features = ["derive", "env"] }
directories = "6.0.0"
fastrand = "2.5.0"
flate2 = "1.1.11"
html2text = { version = "0.17.4", features = ["css"] }
jiff = { version = "0.2.38", features = ["serde"] }
keyring = "4.2.0"
ratatui = { version = "0.30.2", features = ["unstable-rendered-line-info"] }
reqwest = { version = "0.13.5", features = ["json", "form", "query"] }
rusqlite = { version = "0.40", features = ["bundled"] }
rusqlite_migration = "2.6.0"
self-replace = "1.5.1"
semver = { version = "1.0.28", features = ["serde"] }
serde = { version = "1.0.229", features = ["derive"] }
serde_json = "1.0.154"
sha2 = "0.11.1"
tar = "0.4.46"
thiserror = "2.0.21"
tokio = { version = "1.54", features = ["macros", "rt-multi-thread", "time", "sync"] }
zip = { version = "9.4.1", default-features = false, features = ["deflate"] }

[dev-dependencies]
futures = "0.3"
insta = "1.49.0"
tempfile = "3"
wiremock = "0.6.5"
```

| Crate | Why |
| --- | --- |
| `ratatui` (re-exports `crossterm`) | UI, key and mouse events; use `ratatui::crossterm`, no direct `crossterm` dependency |
| `tokio`, `reqwest` | Engine tasks and Graph HTTP |
| `rusqlite` + `rusqlite_migration` | Per-account cache with numbered SQL migrations |
| `keyring` | Refresh token in the OS keychain |
| `jiff` | Timestamps, civil dates, time zones |
| `html2text` | Task notes that arrive as HTML (tasks made from flagged mail) |
| `clap` | CLI with env fallbacks |
| `self-replace`, `semver`, `sha2`, `tar`, `flate2`, `zip` | `todo update` |
| `insta`, `wiremock`, `tempfile`, `futures` | Tests |

Dropped from tuilook: `base64` (attachments). Add the `[target.'cfg(windows)'.build-dependencies] winresource` block only together with an icon.

## 4. Microsoft Graph To Do API

Everything goes through Graph v1.0 (`https://graph.microsoft.com/v1.0`) with delegated scopes `offline_access User.Read Tasks.ReadWrite`. Sources: [todoTask](https://learn.microsoft.com/en-us/graph/api/resources/todotask), [todoTask: delta](https://learn.microsoft.com/en-us/graph/api/todotask-delta), [todoTaskList: delta](https://learn.microsoft.com/en-us/graph/api/todotasklist-delta), [To Do API overview](https://learn.microsoft.com/en-us/graph/api/resources/todo-overview), [delta query overview](https://learn.microsoft.com/en-us/graph/delta-query-overview). Rows marked *verify* come from general Graph knowledge, not from those pages; confirm each against its method page before coding it.

### Sign-in

Identical to tuilook: device authorization grant against `https://login.microsoftonline.com/{tenant}/oauth2/v2.0/{devicecode,token}`, public client, refresh token in the keychain under the account's address, access token cached in memory and refreshed 5 minutes before expiry, `invalid_grant` clears the secret and reports `NotSignedIn`. After sign-in, `GET /me` gives the address (`mail`, else `userPrincipalName`).

- `DEFAULT_TENANT = "consumers"`.
- `DEFAULT_CLIENT_ID`: needs an Entra app registration that allows personal accounts and public client flows. Decision for Christian: a new `tuilist` registration (cleaner consent screen) or reuse tuilook's `5de297ab-80a5-4cfc-bdea-38e33b917993` with `Tasks.ReadWrite` added. Until decided, read it from one `const` in `main.rs`.

### Endpoints

| Operation | Request | Notes |
| --- | --- | --- |
| Who am I | `GET /me?$select=mail,userPrincipalName` |  |
| Sync lists | `GET /me/todo/lists/delta` | Follow `@odata.nextLink` until `@odata.deltaLink`; store the delta link |
| Create list | `POST /me/todo/lists` `{"displayName": "..."}` | *verify* |
| Rename list | `PATCH /me/todo/lists/{listId}` `{"displayName": "..."}` | *verify* |
| Delete list | `DELETE /me/todo/lists/{listId}` | Permanent. *verify* that well-known lists refuse |
| Sync tasks | `GET /me/todo/lists/{listId}/tasks/delta` | Header `Prefer: odata.maxpagesize=50`; same link handling, one delta link per list |
| Create task | `POST /me/todo/lists/{listId}/tasks` | Returns the task |
| Update task | `PATCH /me/todo/lists/{listId}/tasks/{taskId}` | Send only changed fields |
| Delete task | `DELETE /me/todo/lists/{listId}/tasks/{taskId}` | Permanent |
| List steps | `GET .../tasks/{taskId}/checklistItems` | Fetched when a task is first shown, then cached |
| Create step | `POST .../tasks/{taskId}/checklistItems` `{"displayName": "..."}` |  |
| Update step | `PATCH .../checklistItems/{id}` `{"isChecked": true}` or `{"displayName": "..."}` | *verify* |
| Delete step | `DELETE .../checklistItems/{id}` | *verify* |

There is no move endpoint (*verify*). Moving a task = create a copy in the target list (title, body, importance, status, due, reminder, then each step), and delete the original only after every create succeeded. The task gets a new id; recurrence and attachments are lost, so the move dialog warns when the task has either.

### Resources

| Resource | Fields used |
| --- | --- |
| `todoTaskList` | `id`, `displayName`, `isOwner`, `isShared`, `wellknownListName` (`none`, `defaultList`, `flaggedEmails`, `unknownFutureValue`) |
| `todoTask` | `id`, `title`, `status` (`notStarted`, `inProgress`, `completed`, `waitingOnOthers`, `deferred`), `importance` (`low`, `normal`, `high`), `body` (`content`, `contentType` `text` or `html`), `dueDateTime`, `reminderDateTime`, `isReminderOn`, `completedDateTime`, `recurrence`, `hasAttachments`, `categories`, `createdDateTime`, `lastModifiedDateTime` |
| `checklistItem` | `id`, `displayName`, `isChecked`, `createdDateTime`, `checkedDateTime` |
| `dateTimeTimeZone` | `{"dateTime": "2026-11-20T00:00:00.0000000", "timeZone": "UTC"}` |

### Rules that are easy to get wrong

- Deleted items come back in a delta page as `{"id": "...", "@removed": {...}}`; deserialize with `#[serde(rename = "@removed")] removed: Option<IgnoredAny>` as tuilook's `graph/dto.rs` does.
- An expired delta link answers `410 Gone`. Map it to `BackendError::DeltaExpired`, clear the stored link, drop the list's cached tasks and sync from scratch (tuilook `graph/backend.rs` line 299).
- Never edit a `nextLink` or `deltaLink`; pass it back whole. `GraphClient::url` already accepts absolute URLs.
- A task's `id` changes when it moves to another list. Never keep ids outside the per-account cache except in `session.json`, where a miss is harmless.
- Completing: `PATCH {"status": "completed"}`; reopening: `{"status": "notStarted"}`. The server sets and clears `completedDateTime` (*verify*). Treat every status other than `completed` as open.
- Completing a recurring task makes the server create the next occurrence. After any status change on a task with `recurrence`, sync that list at once.
- Star = `importance: "high"`; unstar = `"normal"`. Show `low` as unstarred.
- Due dates are whole days. Write `{"dateTime": "YYYY-MM-DDT00:00:00", "timeZone": "<system IANA zone>"}`; read by converting the returned value from its `timeZone` to the system zone and taking the date. Clear with `"dueDateTime": null`. *Verify* round-trips against the To Do web app in a zone east and west of UTC before release.
- Reminder: set `reminderDateTime` and `isReminderOn: true` together; clear with `isReminderOn: false`.
- Write notes as `{"content": "...", "contentType": "text"}`. Show HTML notes through `html2text`, as tuilook's `core/body.rs` does; editing an HTML note saves it as text, and the editor says so in a dim line above the notes.
- Throttling and retries: keep tuilook's `RetryPolicy` (5 attempts, 1 s base, 60 s cap, honour `Retry-After`, retry 429 always and 5xx only for GET, one re-auth on 401, at most 4 requests at once).

## 5. Architecture

One engine task per account owns that account's SQLite store and talks to Graph; the UI thread only reads the store and exchanges messages with the engines.

```text
keys/mouse -> Keymap -> Action -> AppState::on_key -> Effect
                                                        |
                         TaskBoxes::apply(effect) ------+--> store reads (load list, search)
                                |                       \--> clipboard, session, update
                     Command (mpsc::UnboundedSender, one per account)
                                v
                core::run_engine(backend, store, commands, events)
                    |  TodoBackend (GraphBackend | FakeBackend)
                    |  Store writes
                    v
                Event (mpsc::Sender) -> take_core_events -> AppState -> ui::draw
```

### Layout

```text
src/
  main.rs            CLI (clap), runtime, terminal setup, event loop, wiring
  lib.rs             pub mod accounts, auth, core, graph, model, store, tui, update
  accounts.rs        Account { address, tenant, client_id }, Registry (accounts.json)
  update.rs          release check, download, verify, self-replace
  auth/              mod, http, oauth, provider, secrets        (copied)
  graph/
    mod.rs           pub use GraphBackend, GraphClient, GraphError, RetryPolicy, GRAPH_V1
    client.rs        get, post, post_no_content, patch, delete, retry   (copied)
    dto.rs           serde structs for lists, tasks, steps, delta pages
    backend.rs       impl TodoBackend for GraphBackend
  model/
    mod.rs           re-exports; no IO in this module
    list.rs          TaskList, ListId, WellKnown, sort_lists
    task.rs          Task, TaskId, Status, Importance, TaskChange, TaskDraft, sort_tasks
    step.rs          Step, StepId
    backend.rs       TodoBackend trait, BackendError, DeltaPage<T>
    action.rs        Action, Event
  store/
    mod.rs           Store: open, open_in_memory, queries
    migrations/001_lists_tasks_steps.sql
  core/
    mod.rs           pub use Command, run_engine, sync_lists, sync_tasks
    engine.rs        Command, run_engine
    sync.rs          sync_lists, sync_tasks, SyncError
    changes.rs       optimistic change + revert helpers
    body.rs          notes text from text or HTML
    fake.rs          FakeBackend, fake::list(id), fake::task(id)
  tui/
    mod.rs
    app.rs           AppState, Pane, Effect, Confirm, on_key, on_mouse
    keymap.rs        Keymap (pending g), key -> Action
    run_loop.rs      take_core_events, take_update, take_account_news, on_terminal_event
    taskboxes.rs     per-account store + command sender; apply(Effect)   (tuilook: mailboxes.rs)
    ui.rs            draw, panes, viewport, hit, status_line, dialogs
    list_pane.rs     Smart + Lists sections, rows, counts
    task_list.rs     rows, groups, marks
    groups.rs        Completed group, Planned date groups, fold state
    details.rs       details pane: fields, steps, notes viewer
    task_form.rs     edit form: title, due, reminder, starred, notes
    list_edit.rs     name prompt for a list, a task title or a step
    mover.rs         pick a list to move tasks to
    search.rs  help.rs  dialog.rs  screen.rs  session.rs  demo.rs
    account_manager.rs  clipboard.rs  input.rs  filter_list.rs
    vim.rs  editor.rs  numbers.rs  viewer.rs
tests/
  support/mod.rs  auth_*.rs  graph_*.rs  core_*.rs  store.rs  tui_*.rs  update.rs  snapshots/
```

### Model

```rust
pub struct ListId(pub String);  pub struct TaskId(pub String);  pub struct StepId(pub String);

pub enum WellKnown { DefaultList, FlaggedEmails }
pub struct TaskList { pub id: ListId, pub name: String, pub well_known: Option<WellKnown>,
                      pub is_owner: bool, pub is_shared: bool,
                      pub open: u32 /* counted by the store */ }

pub enum Status { NotStarted, InProgress, Completed, WaitingOnOthers, Deferred }
pub enum Importance { Low, Normal, High }
pub struct Task {
    pub id: TaskId, pub list: ListId, pub title: String,
    pub status: Status, pub importance: Importance,
    pub due: Option<jiff::civil::Date>,
    pub reminder: Option<jiff::Timestamp>,      // None when isReminderOn is false
    pub completed_at: Option<jiff::Timestamp>,
    pub created_at: jiff::Timestamp, pub modified_at: jiff::Timestamp,
    pub notes: String, pub notes_html: bool,
    pub recurring: bool, pub has_attachments: bool,
    pub steps_done: u32, pub steps_total: u32,  // from the store, 0/0 until steps are fetched
}
pub struct Step { pub id: StepId, pub task: TaskId, pub name: String, pub checked: bool,
                  pub created_at: jiff::Timestamp }
/// What a create or an edit sends. None leaves a field alone; Some(None) clears it.
pub struct TaskDraft { pub title: Option<String>, pub starred: Option<bool>, pub completed: Option<bool>,
                       pub due: Option<Option<jiff::civil::Date>>,
                       pub reminder: Option<Option<jiff::Timestamp>>, pub notes: Option<String> }

pub struct DeltaPage<T, Id> { pub upserted: Vec<T>, pub removed: Vec<Id>, pub delta_link: Option<String> }

pub enum BackendError { DeltaExpired, NotFound, Other(String) }

pub trait TodoBackend {   // every fn returns impl Future<Output = Result<_, BackendError>> + Send
    fn lists_delta(&self, link: Option<&str>) -> DeltaPage<TaskList, ListId>;
    fn tasks_delta(&self, list: &ListId, link: Option<&str>) -> DeltaPage<Task, TaskId>;
    fn create_list(&self, name: &str) -> TaskList;
    fn rename_list(&self, list: &ListId, name: &str) -> ();
    fn delete_list(&self, list: &ListId) -> ();
    fn create_task(&self, list: &ListId, draft: &TaskDraft) -> Task;
    fn update_task(&self, list: &ListId, task: &TaskId, draft: &TaskDraft) -> Task;
    fn delete_task(&self, list: &ListId, task: &TaskId) -> ();
    fn steps(&self, list: &ListId, task: &TaskId) -> Vec<Step>;
    fn create_step(&self, list: &ListId, task: &TaskId, name: &str) -> Step;
    fn update_step(&self, list: &ListId, task: &TaskId, step: &StepId, name: Option<&str>, checked: Option<bool>) -> ();
    fn delete_step(&self, list: &ListId, task: &TaskId, step: &StepId) -> ();
}
```

`lists_delta` and `tasks_delta` follow every `nextLink` internally and return one merged page, as tuilook's `delta_sync` does.

### Store (`001_lists_tasks_steps.sql`)

```sql
CREATE TABLE meta  (key TEXT PRIMARY KEY, value TEXT NOT NULL);       -- 'lists_delta_link'
CREATE TABLE lists (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, well_known TEXT,
    is_owner INTEGER NOT NULL, is_shared INTEGER NOT NULL, delta_link TEXT);
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    list_id TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    title TEXT NOT NULL, status TEXT NOT NULL, importance TEXT NOT NULL,
    due TEXT,                      -- YYYY-MM-DD
    reminder_at INTEGER, completed_at INTEGER,
    created_at INTEGER NOT NULL, modified_at INTEGER NOT NULL,
    notes TEXT NOT NULL, notes_html INTEGER NOT NULL,
    recurring INTEGER NOT NULL, has_attachments INTEGER NOT NULL,
    steps_loaded INTEGER NOT NULL DEFAULT 0);
CREATE INDEX tasks_by_list ON tasks(list_id, created_at DESC);
CREATE TABLE steps (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    name TEXT NOT NULL, checked INTEGER NOT NULL, created_at INTEGER NOT NULL);
CREATE INDEX steps_by_task ON steps(task_id, created_at);
```

`Store::open` sets `journal_mode = WAL` and `foreign_keys = ON`, then runs migrations, as tuilook does. Queries: `upsert_lists`, `remove_lists`, `lists` (with open counts), `upsert_tasks`, `remove_tasks`, `clear_tasks_in`, `tasks_in(list)`, `important()`, `planned()`, `all_open()`, `task(id)`, `change_task(id, &TaskDraft)`, `put_steps(task, &[Step])`, `steps(task)`, `upsert_step`, `remove_step`, `search(query, limit)` (case-insensitive `LIKE` on title and notes), `delta_link(list)`, `set_delta_link`, `lists_delta_link`, `set_lists_delta_link`. An upsert that changes `modified_at` sets `steps_loaded = 0`.

### Engine

```rust
pub enum Command {
    SyncAll,                       // lists, then every list's tasks
    SyncList(ListId),
    LoadSteps { list: ListId, task: TaskId },
    CreateList(String), RenameList { list: ListId, name: String }, DeleteList(ListId),
    CreateTask { list: ListId, draft: TaskDraft },
    UpdateTasks { tasks: Vec<(ListId, TaskId)>, draft: TaskDraft },
    DeleteTasks(Vec<(ListId, TaskId)>),
    MoveTasks { tasks: Vec<(ListId, TaskId)>, to: ListId },
    CreateStep { list: ListId, task: TaskId, name: String },
    UpdateStep { list: ListId, task: TaskId, step: StepId, name: Option<String>, checked: Option<bool> },
    DeleteStep { list: ListId, task: TaskId, step: StepId },
}
pub enum Event {
    ListsSynced, ListSynced(ListId), SyncFinished,
    TasksChanged(Vec<ListId>),     // the store changed; reload what is shown
    StepsLoaded(TaskId),
    Failed { what: String, reason: String },   // already reverted in the store
    SignedOut,                     // refresh failed with NotSignedIn
}
```

- `run_engine(backend: impl TodoBackend, store: Store, commands: UnboundedReceiver<Command>, events: Sender<Event>)` loops until the sender is dropped.
- Writes are optimistic: apply to the store, emit `TasksChanged`, call the backend; on error restore the previous rows, emit `TasksChanged` and `Failed`. On success upsert what the backend returned.
- `sync_tasks` on `DeltaExpired`: clear link and tasks for that list, retry once with no link.
- `SyncAll` continues past a failing list and still ends with `SyncFinished`; the status line shows the first error.

### Event loop (`main.rs`)

Same shape as tuilook: `event::poll(Duration::from_millis(100))`, then drain core events, update news and account news, then draw. A timer sends `Command::SyncAll` to every account every `--sync-every` seconds and 3 s after any write. Only `KeyEventKind::Press` is handled (Windows also reports releases). Mouse capture is on: click focuses a pane and selects a row, wheel scrolls. A panic hook and normal exit both restore the terminal and save `session.json`.

## 6. UI

Three panes side by side, as in tuilook: lists (two stacked sections), tasks, details; one status line below. Styles are in section 2.

```text
┌ Smart ───────────┐┏ Groceries ━━━━━━━━━━━━━━━━━━━━━━━┓┌ Details ─────────────────────────┐
│  Important (2)  │┃> [ ] Milk               ★ Fri 20┃│1   Milk                          │
│  Planned (5)    │┃  [ ] Coffee beans   1/3  Today ┃│  1 [ ] Not completed  ★ Starred  │
│  All (14)       │┃  [ ] Bread          ↻          ┃│  2 Due: Fri 20 Nov 2026          │
└─────────────────┘┃  ▾ Completed (2)               ┃│  3 Remind: Fri 20 Nov, 09:00     │
┌ Lists ───────────┐┃  [x] Eggs                      ┃│  4                               │
│  Tasks (6)      │┃  [x] Butter                    ┃│  5 Steps                         │
│> Groceries (3)  │┃                                ┃│  6 [x] Check the fridge          │
│  Work (5)       │┃                                ┃│  7 [ ] Semi-skimmed              │
│  Flagged email  │┃                                ┃│  8                               │
└─────────────────┘┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛│  9 Notes                         │
 ? help · Tab switch pane · a add · x complete · e edit · / search · d delete · q quit
```

### Layout rules

- Widths: lists pane 18 columns at 90 wide, as tuilook's folder pane; tasks and details share the rest. Copy `panes()`, `viewport()` and `hit()` from tuilook's `ui.rs` and adapt.
- `z` zooms the focused pane to the whole area. `b` hides the lists pane. `L` puts details below tasks; pressing it again puts it back right.
- With several accounts, each account's address heads its own lists in the Lists section and folds with `h` / `l`. With one account no heading is shown.
- Too small (under 40 × 8): draw only `Terminal too small`.

### Lists pane

- **Smart** section, computed from the cache across all accounts: `Important` (open and starred), `Planned` (open with a due date), `All` (every open task). Counts in parentheses; no count when zero.
- **Lists** section: the default list first (shown by its server name, normally `Tasks`), then other lists by name ignoring case, then `Flagged email` last when present. Count = open tasks. Shared lists get a dim `  ⇄ ` after the name.
- Moving the selection loads that list's tasks at once from the store; `Enter` also moves focus to the tasks pane.
- Empty state in Lists: dim `a makes a list`.

### Tasks pane

- Title = the list's name. One row per task: ` >  ` marker, `[ ]` or `[x]`, title, then right-aligned in this order when present: steps `1/3` (dim), `↻` recurring (dim), `★` starred (Yellow), due date.
- Due date text: `Today`, `Tomorrow`, `Yesterday`, weekday + day within the next 6 days (`Fri 20`), else `20 Nov`, with the year when it is not this year. Overdue dates are Red; others DarkGray.
- Completed rows are DarkGray with `Modifier::CROSSED_OUT` on the title.
- Marked rows show `●` in place of the space before the checkbox, and the title bar shows `  (3 marked) `, as tuilook does for mail.
- Order in a list: open tasks newest first by `created_at`, then a `▾ Completed (n)` group, newest completion first. The group folds with `h` and unfolds with `l` or `Enter` on its header; `c` hides or shows it altogether. Fold and hide states persist.
- `Planned` groups: `Overdue`, `Today`, `Tomorrow`, `This Week`, `Later`, each foldable, ordered by due date. `Important` and `All` group by list name. Smart views show a dim list name after the title when it fits.
- Empty state: dim `No tasks. a adds one`.
- Titles and notes pass through tuilook's `shown()` so control and zero-width characters cannot break the layout; widths use `Line::width`.

### Details pane

- Read-only vim viewer (tuilook's `viewer.rs` + `reader.rs`): cursor, hybrid line numbers, `v` / `V` / `y`, mode badge `NORMAL` in the status line while focused.
- Content, top to bottom: title; `[ ] Not completed` or `[x] Completed <date>` and `★ Starred` when starred; `Due:`; `Remind:`; `Repeats` when recurring; `List:` in smart views; blank; `Steps` and one `[ ]` / `[x]` row per step; blank; `Notes` and the notes text wrapped; blank; dim `Created <date> · Changed <date>`. Lines with nothing to show are left out.
- Dates use the system time zone: `Fri 20 Nov 2026`, times `09:00`.
- When the cursor is on a step row, `x`, `F2` and `d` act on that step; elsewhere they act on the task.
- Selecting a task with `steps_loaded = 0` sends `LoadSteps`; show dim `Loading steps…` under `Steps` until `StepsLoaded`.

### Dialogs (one at a time, each takes every key until it closes)

| Dialog | Opens on | Looks like |
| --- | --- | --- |
| Help | `?` | Two columns, keys Cyan, width 76, closes on any key; table depends on the screen |
| Name prompt | `a`, `A`, `s`, `F2` | Yellow border, 52 wide, title `New list` / `New task` / `New step` / `Rename`, ` Name:  ` + `TextInput`, hint `Enter save   Esc cancel` |
| Due prompt | `T` | Same prompt, label ` Due:  `, accepts `2026-11-30`, `today`, `tomorrow`, `mon`..`sun`, `+3` (days); empty clears; a bad value shows Red text and stays open |
| Move picker | `m` | tuilook's move picker over the lists of the task's account, ` Folder:  ` label becomes ` List:  `, 12 rows, type to filter |
| Confirm delete | `d` | Red border, `Delete 1 task? This cannot be undone.` / `Delete Groceries and its 3 tasks? This cannot be undone.`, `y / Enter delete   Esc or any other key cancel` |
| Confirm discard | `Esc` in a changed form | Red border, `Close the task and lose your changes?` |
| Confirm update | `U` | Yellow border, as tuilook |
| Accounts | `M` | tuilook's account manager minus signatures |

### Task form (`e`)

Takes over the tasks and details area with a Thick border, title `Edit task`, like tuilook's appointment form. Fields: `Title`, `Due` (same syntax as the due prompt), `Remind` (`2026-11-30 09:00`, empty for none), `Starred` (`Space` toggles), then a rule and `Notes` edited with the vim `Editor`. `Tab` / `Shift-Tab` / `Enter` / `↑` `↓` move between fields, `Ctrl-S` saves only the fields that changed, `Esc` closes (asking if changed). Invalid dates show Red under the field and block the save.

### Search (`/`)

As tuilook: the status line becomes `  /query ` with a cursor; the tasks pane shows matches from every account's cache as you type (title or notes contain the text, ignoring case), with the list name after each. `Enter` moves into the results, `Esc` returns to the list you were in. No results: dim `Nothing matches`.

### Status line

- Left to right: update badge, mode badge (only when details is focused or an editor is open), then one of: search query, error (Red), status (`Syncing…`, `Synced`, `Deleted 2 tasks`), or key hints for the focused pane.
- Hints: lists pane `  ? help · Tab switch pane · Enter open · a new list · F2 rename · d delete · q quit `; tasks pane as in the mock; details `  ? help · x complete · s add step · i edit · v select · y copy · q quit `.
- A unit test asserts every key named in a hint or help row is bound, as tuilook's `help.rs` tests do.

## 7. Keybindings

Keys shared with tuilook keep their meaning; task keys fill the letters tuilook uses for mail. `Keymap` stays a pure `KeyEvent -> Option<Action>` function with a pending `g`, and each dialog, form and editor handles its own keys.

### Every pane

| Keys | What they do |
| --- | --- |
| `Tab` / `Shift-Tab` | Next pane / previous pane |
| `j` `k` or `↓` `↑` | Move |
| `l` `h` or `→` `←` | Unfold a group or account, fold it or go to its header |
| `gg` `G` or `Home` `End` | Jump to the first or last |
| `PgDn` `PgUp` or `Ctrl-F` `Ctrl-B` | Move a page |
| `Ctrl-D` `Ctrl-U` | Move half a page |
| `Enter` | Open the list, move to the task's details, or fold or unfold a group |
| `z` | Zoom the focused pane, or show every pane again |
| `b` | Hide or show the lists (not in details, where `b` moves back a word) |
| `L` | Move details below the tasks, or back right of them |
| `/` | Search every list of every account |
| `F5` or `Ctrl-R` | Sync now |
| `M` | Manage accounts |
| `U` | Update TUIList to its latest release |
| `?` | Show the keys |
| `q` or `Ctrl-C` | Quit |

### Lists pane

| Keys | What they do |
| --- | --- |
| `a` | Make a new list |
| `F2` | Rename the selected list |
| `d` `D` | Delete the selected list and its tasks, after asking |

The default list and `Flagged email` cannot be renamed or deleted; the status line says so. Smart lists take none of these keys.

### Tasks pane

| Keys | What they do |
| --- | --- |
| `a` | Add a task to the list shown, asking its title |
| `x` | Complete the marked (or selected) tasks, or reopen them when all are completed |
| `*` | Star the marked (or selected) tasks, or unstar them when all are starred |
| `e` `i` | Edit the selected task in the form |
| `F2` | Rename the selected task |
| `t` | Make the marked (or selected) tasks due today, or clear the date when all are due today |
| `T` | Ask for a due date |
| `s` | Add a step to the selected task |
| `m` | Move the marked (or selected) tasks to another list |
| `d` `D` | Delete the marked (or selected) tasks, after asking |
| `y` | Copy the selected task's title |
| `c` | Hide or show completed tasks |
| `Space` | Mark or unmark the selected task, then move down |
| `Ctrl-A` | Mark every task shown |
| `Esc` | Clear the marks; in search results with no marks, leave the search |

`a` in a smart list adds to the default list of the first account: starred in `Important`, due today in `Planned`. Tasks can only be moved within their own account.

### Details pane

| Keys | What they do |
| --- | --- |
| `h` `j` `k` `l` `w` `b` `e` `0` `^` `$` `gg` `G` | Move the cursor |
| `v` / `V`, then `y` | Pick characters / lines, copy them |
| `yy` `yw` `y$` | Copy a line, a word, to the end of the line |
| `→` `←` | Show the next or previous task |
| `x` | Check or uncheck the step under the cursor; elsewhere complete or reopen the task |
| `s` | Add a step |
| `F2` | Rename the step under the cursor; elsewhere the task |
| `d` | Delete the step under the cursor, after asking; elsewhere the task |
| `i` | Edit the task in the form |
| `*` `t` `T` `m` | As in the tasks pane, for this task |

### Task form and notes editor

| Keys | What they do |
| --- | --- |
| `Tab` / `Shift-Tab`, `Enter`, `↓` `↑` | Next field / previous field |
| `Space` on `Starred` | Star or unstar |
| `Ctrl-S` | Save |
| `Esc` (in a field or in normal mode) | Close, asking first if anything changed |
| `?` (in normal mode) | Show the keys |
| `i` `a` `I` `A` `o` `O`, `Esc` | Insert mode, back to normal mode |
| Motions, `x` `X` `D` `C` `J` `r`, `d` `c` `y` + motion, `p` `P`, `u` `Ctrl-R`, `v` `V`, counts | As in tuilook's message body; the copied `vim.rs` and `editor.rs` provide all of it |

Text prompts use tuilook's `TextInput`: arrows, `Home`, `End`, `Backspace`, `Delete`, `Enter`, `Esc`.

### `Action`

```rust
pub enum Action {
    FocusNextPane, FocusPrevPane, SelectNext, SelectPrev, PageDown, PageUp, HalfPageDown, HalfPageUp,
    Top, Bottom, Open, Expand, Collapse, Quit, ToggleZoom, ToggleLists, ToggleDetailsPlace,
    ToggleMark, MarkAll, ClearMarks, Refresh, Help, Search, Update, ManageAccounts,
    Add,            // a: list in the lists pane, task in the tasks pane
    Rename,         // F2
    Delete,         // d, D
    ToggleComplete, // x
    ToggleStar,     // *
    Edit,           // e, i
    DueToday,       // t
    AskDue,         // T
    AddStep,        // s
    Move,           // m
    Copy,           // y
    ToggleCompletedShown, // c
}
```

Keep tuilook's keymap unit tests for the shared keys and add one test per new binding.

## 8. CLI, configuration and storage

The CLI mirrors `otlk`: options with env fallbacks, no config file.

| Command | What it does |
| --- | --- |
| `todo` | Open the client with every signed-in account |
| `todo login` | Sign in with a device code; run again to add an account |
| `todo logout [address]` | Forget an account's sign-in and cached tasks; the address may be a unique prefix, and may be left out with one account |
| `todo accounts` | List the signed-in accounts |
| `todo sync` | Sync every account once, without the UI |
| `todo check-update` | Ask GitHub whether a newer release exists |
| `todo update` | Replace `todo` with the latest release |

| Option | Environment variable | Default | Meaning |
| --- | --- | --- | --- |
| `--demo` |  |  | Browse built-in sample lists instead of signing in |
| `--sync-every <seconds>` | `TUILIST_SYNC_EVERY` | `60` | Seconds between syncs; `0` syncs only at start and on `F5` |
| `--tenant <tenant>` | `TUILIST_TENANT` | `consumers` | `consumers`, `organizations`, `common` or a tenant ID |
| `--client-id <id>` | `TUILIST_CLIENT_ID` | TUIList's own | Entra ID app (client) ID |
| `--no-update-check` | `TUILIST_NO_UPDATE_CHECK` |  | Don't ask GitHub for a newer release at start |

`#[command(name = "todo", version, about = "A terminal client for Microsoft To Do")]`. Running `todo` with no signed-in account prints `Not signed in. Run todo login, or todo --demo to look around.` and exits with code 1.

| What | Where |
| --- | --- |
| Refresh token | OS keychain, service `tuilist`, one entry per address; keep tuilook's splitting of long secrets with prefix `tuilist-pieces:` |
| Accounts | `accounts.json` in the data folder: `~/.local/share/tuilist`, `~/Library/Application Support/tuilist`, `%APPDATA%\tuilist\data` (`ProjectDirs::from("", "", "tuilist")`) |
| Task cache | `<address>/cache.db` in the data folder; safe to delete |
| Last session | `session.json` in the cache folder: `~/.cache/tuilist`, `~/Library/Caches/tuilist`, `%LOCALAPPDATA%\tuilist\cache` |
| Last update check | `update-check.json` in the cache folder |

`session.json` (`#[serde(default)]`, unknown or missing fields ignored): `focus`, `list` (id or smart-list name), `task`, `zoomed`, `lists_hidden`, `details_below`, `completed_hidden`, `collapsed` (accounts), `collapsed_groups`.

`--demo` uses `FakeBackend` with in-memory stores, never touches the keychain, the data folder or GitHub, and holds two accounts with about 6 lists and 40 tasks covering every state: starred, overdue, due today, recurring, with steps, with long notes, completed, a shared list and `Flagged email`.

## 9. Repo scaffolding

The first commits create the public-ready shell before any feature: manifest, license, README, CHANGELOG, workflows, install scripts and a `todo --version` that builds on all five CI runners.

```text
.devcontainer/devcontainer.json   .gitattributes   .gitignore
.github/workflows/ci.yml  release.yml  pages.yml
CHANGELOG.md  Cargo.toml  Cargo.lock  LICENSE  README.md  build.rs
docs/install.sh  docs/install.ps1
images/            screenshots, added before 0.1.0
src/  tests/
```

- `.gitignore`: `/target` and `*.snap.new`. `.gitattributes`: `* text=auto eol=lf`. `Cargo.lock` is committed.

### CHANGELOG.md

Same format as tuilook and tuicalc: Keep a Changelog 1.1.0, SemVer, `### Added` / `### Changed` / `### Fixed`, link references at the bottom. Entries are written for users in full sentences, name the keys in backticks, and say what the user can now do, never which module changed. Start with exactly:

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

[Unreleased]: https://github.com/christianhelle/tuilist/commits/main
```

At the 0.1.0 release the section becomes `## [0.1.0] - YYYY-MM-DD`, a new empty `## [Unreleased]` goes above it, and the links become:

```markdown
[Unreleased]: https://github.com/christianhelle/tuilist/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/christianhelle/tuilist/releases/tag/v0.1.0
```

The release workflow cuts its notes from this file with `awk` (from the `## [x.y.z]` heading to the next ` ##  ` or link line) and fails if they are empty, so the heading format is load-bearing. Example of the voice to match: "Steps: `s` adds a step to the selected task, and in the details `x` checks it off, `F2` renames it and `d` deletes it."

### README.md

Same order and tone as tuilook's README. Sections:

1. `# TUIList`, three badges (crates.io `tuilist`, CI, MIT), two-sentence pitch, main screenshot `images/tasks.jpg`.
2. `## Install`: "TUIList installs as the `todo` command." Then `### macOS/Linux` (`curl -fsSL https://christianhelle.com/tuilist/install | bash`, with `INSTALL_DIR` and `VERSION`), `### Windows PowerShell` (`irm https://christianhelle.com/tuilist/install.ps1 | iex`, `-InstallDir`, `-Version`), `### Cargo` (`cargo install tuilist --locked`, Rust 1.95 or later), `### Release archives`, install from Git.
3. `## Development Container` and the three CI commands, `cargo run -- --demo`.
4. `## Try it without signing in` (`todo --demo`).
5. `## Sign in`, `### More than one account`, `### Managing accounts in the client`, `### Work or school accounts`: copy tuilook's four numbered steps, replacing the permission list with `User.Read`, `Tasks.ReadWrite`, `offline_access`; keep `#### If device-code sign-in is blocked`.
6. `## The screen`, then `## Lists`, `## Tasks`, `## Steps`, `## Details and copying`, `## Editing a task`, `## Searching`: one key table per section, taken from section 7 of this plan, plus the behaviour notes from section 6.
7. `## Syncing`, `## Updates`, `## Options` (both tables from section 8), `## Where TUIList keeps things`, `## Limitations` (the non-goals, and that a moved task loses its recurrence), `## License`.

Write README sections in the same commit as the feature they describe; until then a section does not exist. No placeholders or "coming soon".

### Workflows

Copy all three from tuilook and replace names only.

| File | Triggers | Jobs | Edits |
| --- | --- | --- | --- |
| `ci.yml` | push to `main`, pull requests | `fmt` (`cargo fmt --all --check`), `clippy` (`cargo clippy --all-targets -- -D warnings`), `test` on `ubuntu-latest`, `ubuntu-24.04-arm`, `macos-latest`, `windows-latest`, `windows-11-arm` | None. Keep the `clang-cl` step for Windows ARM64: `reqwest` pulls `aws-lc-sys` |
| `release.yml` | tags `v*` | `build` for 6 targets (Linux x64/ARM64 gnu, Windows x64/ARM64 msvc, macOS ARM64/x64), `release` (notes from CHANGELOG, `softprops/action-gh-release@v3`), `publish` (tag must equal crate version; skips if already on crates.io; trusted publishing through `rust-lang/crates-io-auth-action@v1`) | `otlk` → `todo` in the Package step and artifact names; `tuilook` → `tuilist` in the crates.io URL, user agent and messages |
| `pages.yml` | push or PR touching `docs/**` or the workflow | Copies `docs/install.sh` to `docs/install`, deploys `docs/` to GitHub Pages | None |

Archive names are `todo-v0.1.0-<target>.tar.gz` (`.zip` on Windows), each holding `todo`, `README.md`, `LICENSE`, `CHANGELOG.md`. `update.rs::asset_name` and both install scripts must agree with that; `tests/update.rs` covers it.

### Install scripts

Copy `docs/install.sh` and `docs/install.ps1` from tuilook; set `GITHUB_REPO="christianhelle/tuilist"`, `BINARY_NAME="todo"`, `DOCUMENTATION_URL`, and the Windows default folder `%LOCALAPPDATA%\Programs\tuilist`.

### One-time setup only Christian can do

- [ ] Create `github.com/christianhelle/tuilist` with labels including `enhancement`.
- [ ] Enable GitHub Pages with source "GitHub Actions" so `christianhelle.com/tuilist/install` resolves.
- [ ] Reserve the `tuilist` crate and add the repo's `release.yml` as a crates.io trusted publisher.
- [ ] Provide the Entra client ID (section 4).
- [ ] Optional: `assets/icon.ico` and `assets/icon.png` for the Windows binary.

## 10. Testing

Every layer below `main.rs` is tested without a network, a keychain or a real terminal, as in tuilook; CI runs the same three commands on five runners.

| Layer | Test file(s) | How |
| --- | --- | --- |
| Keymap, help, input, filter list, vim, editor, groups, date text and due parsing | `#[cfg(test)]` in the module | Plain unit tests |
| Auth | `tests/auth_device_code.rs`, `auth_provider.rs`, `auth_refresh.rs`, `keyring_long.rs` | Copied; `wiremock` token endpoint, recording `Sleeper`, `MemoryStore` |
| Graph client | `tests/graph_client.rs` | Copied: bearer header, 401 re-auth once, 429 `Retry-After`, 5xx retried for GET only, error body decoding |
| Graph backend | `tests/graph_lists.rs`, `graph_tasks.rs`, `graph_delta.rs`, `graph_steps.rs`, `graph_me.rs` | `wiremock` with JSON bodies shaped like the Microsoft Learn examples; assert method, path, headers and request body; paging over `nextLink`; `@removed`; 410 → `DeltaExpired`; date and enum mapping |
| Store | `tests/store.rs` | `Store::open_in_memory()`; upsert, remove, cascade, counts, smart queries, search, delta links; `migrations_are_valid` |
| Core | `tests/core_sync.rs`, `core_engine.rs` | `FakeBackend` + in-memory store; drive `run_engine` with commands, assert events and store; failure reverts; delta expiry resyncs; recurring completion triggers a list sync; move = create then delete, and a failed create deletes nothing |
| TUI state | `tests/tui_app.rs`, `tui_run_loop.rs` | Feed `KeyEvent`s to `AppState`, assert selection, dialogs and returned `Effect`s |
| TUI render | `tests/tui_render.rs` + `tests/snapshots/` | `Terminal::new(TestBackend::new(90, 12))`, `insta::assert_snapshot!(terminal.backend().to_string())`; fixed `app.clock` so dates never depend on the machine |
| Update | `tests/update.rs` | Copied |

**Rules**

- Test names are sentences about behaviour: `completing_a_task_moves_it_under_completed`, `deleting_asks_first`, `a_failed_rename_puts_the_old_name_back`.
- Time is injected: `AppState.clock: Option<Zoned>` for rendering, a `now` argument for due-date logic. No test reads the system clock or time zone.
- One snapshot per visible state: each pane focused, each dialog, each empty state, the form, search with and without results, marks, folded group, zoom, details below, update badge, several accounts, too-small terminal.
- Review snapshots with `cargo insta review`; never commit `*.snap.new`.
- A test in `help.rs` fails when a key in the help table or a status hint has no binding, or a binding has no help row.
- Nothing in tests or `--demo` calls Graph, GitHub or the keychain. Live checks are manual and listed in section 12.

## 11. Implementation phases

Twelve phases, each one pull request that leaves `main` green and usable; do them in order. Within a phase, each numbered step is roughly one test-then-code nano commit or a few.

### Phase 0: Scaffold

1. `cargo init`, `Cargo.toml` from section 3 with only `clap` and `anyhow` for now; add each dependency in the commit that first uses it.
2. `LICENSE`, `.gitignore`, `.gitattributes`, `.devcontainer`, `build.rs` (target line only).
3. `README.md` (title, badges, pitch, Development), `CHANGELOG.md` (header + empty Unreleased).
4. `ci.yml`, `release.yml`, `pages.yml`, `docs/install.sh`, `docs/install.ps1`.
5. `src/lib.rs`, `src/main.rs` with the `Cli` struct and `--version`.

Done when: CI is green on all five runners; `cargo run -- --version` prints `todo 0.1.0`.

### Phase 1: Model, store, fake backend

1. `model/` types and `sort_lists`, `sort_tasks` with unit tests.
2. `store/` migration 001 and every query in section 5, driven by `tests/store.rs`.
3. `TodoBackend`, `BackendError`, `DeltaPage`; `core::fake::FakeBackend` (in-memory lists, tasks, steps, scripted failures) with `fake::list(id)`, `fake::task(id)` builders.

Done when: `tests/store.rs` covers upsert, remove, cascade, counts, smart queries and search.

### Phase 2: Engine and sync

1. `core/sync.rs`: `sync_lists`, `sync_tasks`, delta links, `DeltaExpired` recovery.
2. `core/engine.rs`: `Command`, `Event`, `run_engine`; `SyncAll`, `SyncList`, `LoadSteps`.
3. Write commands one by one, each with its optimistic apply and revert test: task create, update, delete; list create, rename, delete; steps; move.

Done when: `tests/core_sync.rs` and `tests/core_engine.rs` pass against `FakeBackend`, including every failure path.

### Phase 3: Read-only TUI on demo data

1. Copy `input.rs`, `filter_list.rs`, `clipboard.rs`, `numbers.rs`, `vim.rs`, `editor.rs`, `viewer.rs` with their tests.
2. `keymap.rs` (shared keys only), `app.rs` (`AppState`, `Pane`, focus, selection, paging, zoom, hide lists, details below), `ui.rs` (`panes`, `draw`, `section_block`, status line, too-small).
3. `list_pane.rs`, `task_list.rs`, `groups.rs`, `details.rs` with the row formats of section 6.
4. `help.rs` and the help dialog; mouse click and wheel.
5. `demo.rs`, `taskboxes.rs`, `run_loop.rs`, terminal setup, panic hook and event loop in `main.rs`; `todo --demo`.
6. `session.rs`: save on quit, restore at start.

Done when: `todo --demo` browses lists, tasks and details with every key in the "Every pane" table except `/`, `M`, `U`; snapshots exist for each pane focused, empty states, folded group, zoom and details below. CHANGELOG has its first `### Added` lines; README gains "Try it without signing in" and "The screen".

### Phase 4: Editing tasks (still demo)

1. `dialog.rs`, `Confirm`, name prompt (`list_edit.rs`): `a`, `F2`.
2. `x` complete, `*` star, `t` due today, marks (`Space`, `Ctrl-A`, `Esc`) applying to all of them.
3. Due parsing (`today`, `tomorrow`, weekday, `+N`, ISO date) and the `T` prompt.
4. `d` delete with confirm; `c` hide completed; `y` copy.
5. `mover.rs`: `m`.
6. Steps in details: `s`, and `x` / `F2` / `d` on a step row.
7. `task_form.rs`: `e` / `i`, fields, vim notes, `Ctrl-S`, discard confirm.
8. List keys in the lists pane: `a`, `F2`, `d`, with the well-known list refusal.

Done when: every key in section 7 except `/`, `M`, `U` works in `--demo`, each with a state test and a snapshot of its dialog; a scripted `FakeBackend` failure shows the Red status and the row reverts.

### Phase 5: Search

1. `Store::search`, `search.rs`, status-line input with cursor, live results, `Enter` / `Esc`.

Done when: search finds by title and notes across both demo accounts and the snapshots for results and no results pass.

### Phase 6: Auth and accounts

1. Copy `auth/` and its tests; rename keyring service and piece prefix.
2. Copy `accounts.rs` minus signatures; `todo login`, `logout`, `accounts`.
3. `GET /me` in a minimal `GraphClient` use to learn the address.

Done when: the copied auth tests pass and `todo login` signs in to a real personal account (manual check).

### Phase 7: Graph backend

1. Copy `graph/client.rs` and `tests/graph_client.rs`; trim attachment methods.
2. `dto.rs` and mapping to model types, with unit tests for dates, statuses, importance, `@removed`, HTML notes.
3. `backend.rs` method by method, each driven by a `wiremock` test: lists delta, tasks delta with paging and 410, list create/rename/delete, task create/update/delete, steps.
4. Before each method, open its Microsoft Learn page and confirm path, verb and body; resolve every *verify* in section 4.

Done when: all `tests/graph_*.rs` pass and no *verify* note in section 4 is unresolved.

### Phase 8: Live client

1. Wire `GraphBackend` + on-disk `Store` per account in `main.rs`; background sync timer; `F5`; `todo sync`.
2. `SignedOut` event shows `Sign-in expired. Press M to sign in again`.
3. Several accounts in the lists pane with headings.
4. `account_manager.rs` and `M`.

Done when: the manual checklist in section 12 passes against a real account, and changes made in the To Do web app appear within one sync interval.

### Phase 9: Updates

1. Copy `update.rs` and `tests/update.rs`; `todo check-update`, `todo update`, start-up check, `U`, badges, `--no-update-check`.

Done when: `tests/update.rs` passes with the new asset names.

### Phase 10: Polish

1. Narrow and tiny terminals, very long titles, wide characters and emoji, thousands of tasks in one list (render only visible rows).
2. Review every string against tuilook's voice; make help tables and hints complete.
3. Full README with screenshots in `images/` (`tasks.jpg`, `help.jpg`, `edit-task.jpg`, `move-task.jpg`, `details-below.jpg`).

Done when: the help/binding consistency test passes and the README covers every key and option.

### Phase 11: Release 0.1.0

1. Move Unreleased to `## [0.1.0] - <date>`, fix the link references, confirm `Cargo.toml` version.
2. `cargo publish --dry-run --locked`; tag `v0.1.0`; check the release has six archives and notes, crates.io has the crate, and both install one-liners work.

Done when: `cargo install tuilist --locked` and both install scripts give a working `todo`.

## 12. Open questions and manual checks

Five decisions are Christian's; each has a default so implementation is never blocked.

| Question | Default if unanswered |
| --- | --- |
| Entra client ID: new `tuilist` registration, or tuilook's with `Tasks.ReadWrite` added? | A `DEFAULT_CLIENT_ID` const holding tuilook's ID; swap before 0.1.0 |
| Is the crate name `tuilist` free on crates.io, and is a binary called `todo` acceptable given other tools use that name? | Keep both; `cargo install` refuses to overwrite a foreign `todo` unless forced |
| Non-interactive commands (`todo add "Milk" --list Groceries`, `todo ls`)? | Not in 0.1.0 |
| `d` and `D` both delete (To Do has no trash). Keep `D` as a synonym? | Yes, so tuilook habits never do something else |
| Default task order: newest first, or by due date then newest? | Newest first, as the To Do apps add new tasks on top |

**Assumptions to confirm in phase 7** (not stated on the pages read for this plan)

- [ ] Paths, verbs and bodies for list create/rename/delete and checklist item update/delete.
- [ ] Whether `tasks/delta` reports a task when only its steps change; if not, reload steps each time a task is shown and on `F5`.
- [ ] Whether `$expand=checklistItems` on `tasks/delta` works, which would give step counts without one request per task.
- [ ] How the server answers deleting or renaming `defaultList` and `flaggedEmails`.
- [ ] Due-date round trip across time zones, and whether `Prefer: outlook.timezone` is needed on reads.
- [ ] That personal accounts can consent to `Tasks.ReadWrite` with the chosen client ID without it being listed on the registration.
- [ ] The error code and status when a delta link is too old (`410` assumed, as for mail).

**Manual checklist before 0.1.0, against a real account and the To Do web app**

- [ ] Sign in, quit, start again: no second sign-in, cached lists show before the sync ends.
- [ ] Create, rename, star, complete, reopen, set and clear due, edit notes, add and check steps, move, delete: each shows in the web app, and the reverse within one sync.
- [ ] Complete a recurring task: the next occurrence appears.
- [ ] A task created from flagged mail shows readable notes.
- [ ] Unplug the network: cached data browses, a write shows a Red error and reverts, sync resumes when back.
- [ ] Two accounts: lists head correctly, moving is offered only within an account, `logout` removes one and its cache.
- [ ] Windows Terminal, a macOS terminal and a Linux terminal: borders, `★`, `↻`, `▾` render; `y` copies over SSH (OSC 52).

## Sources

- [christianhelle/tuilook](https://github.com/christianhelle/tuilook) and [christianhelle/tuicalc](https://github.com/christianhelle/tuicalc), `main` as cloned on 2026-11-13
- [todoTask resource type](https://learn.microsoft.com/en-us/graph/api/resources/todotask)
- [todoTask: delta](https://learn.microsoft.com/en-us/graph/api/todotask-delta)
- [todoTaskList: delta](https://learn.microsoft.com/en-us/graph/api/todotasklist-delta), [To Do API overview](https://learn.microsoft.com/en-us/graph/api/resources/todo-overview), [delta query overview](https://learn.microsoft.com/en-us/graph/delta-query-overview): search excerpts only, not opened in full
