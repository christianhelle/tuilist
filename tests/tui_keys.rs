use jiff::Zoned;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tuilist::{
    core::{Command, sync_lists, sync_tasks},
    store::Store,
    tui::{
        app::{AppState, Effect, Pane, Status},
        demo,
        dialog::Dialog,
        keymap::Keymap,
        taskboxes::TaskBoxes,
    },
};

fn demo_app() -> (AppState, TaskBoxes) {
    let now: Zoned = "2026-11-13T09:00:00[UTC]".parse().unwrap();
    let mut accounts = Vec::new();
    futures::executor::block_on(async {
        for (address, backend) in demo::accounts_at(now.clone()) {
            let mut store = Store::open_in_memory().unwrap();
            sync_lists(&backend, &mut store).await.unwrap();
            for list in store.lists().unwrap() {
                sync_tasks(&backend, &mut store, &list.id).await.unwrap();
            }
            accounts.push((address, store));
        }
    });
    let mut app = AppState {
        clock: Some(now),
        ..AppState::default()
    };
    let mut boxes = TaskBoxes::new(accounts);
    boxes.load(&mut app).unwrap();
    (app, boxes)
}

fn press(app: &mut AppState, code: KeyCode) -> Effect {
    app.on_key(KeyEvent::from(code), &mut Keymap::default())
}

fn ctrl(app: &mut AppState, c: char) -> Effect {
    app.on_key(
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL),
        &mut Keymap::default(),
    )
}

fn sent(effect: Effect) -> Command {
    match effect {
        Effect::Send { command, .. } => command,
        other => panic!("expected a command, got {other:?}"),
    }
}

#[test]
fn x_completes_the_selected_task() {
    let (mut app, _boxes) = demo_app();
    app.focus = Pane::Tasks;

    let Command::UpdateTasks { tasks, draft } = sent(press(&mut app, KeyCode::Char('x'))) else {
        panic!("x should update a task");
    };

    assert_eq!(tasks.len(), 1);
    assert_eq!(draft.completed, Some(true));
}

#[test]
fn star_and_due_today_send_single_updates() {
    let (mut app, _boxes) = demo_app();
    app.focus = Pane::Tasks;

    let starred = app.selected_task().unwrap().task.importance == tuilist::model::Importance::High;
    let Command::UpdateTasks { draft, .. } = sent(press(&mut app, KeyCode::Char('*'))) else {
        panic!("* should update a task");
    };
    assert_eq!(draft.starred, Some(!starred));

    let was_today = app.selected_task().unwrap().task.due == Some(app.today());
    let Command::UpdateTasks { draft, .. } = sent(press(&mut app, KeyCode::Char('t'))) else {
        panic!("t should update a task");
    };
    let expected = if was_today {
        Some(None)
    } else {
        Some(Some(app.today()))
    };
    assert_eq!(draft.due, expected);
}

#[test]
fn delete_asks_before_removing_a_task() {
    let (mut app, _boxes) = demo_app();
    app.focus = Pane::Tasks;

    assert!(matches!(press(&mut app, KeyCode::Char('d')), Effect::None));
    assert!(matches!(app.dialog, Some(Dialog::Confirm(_))));
}

#[test]
fn move_offers_the_lists() {
    let (mut app, _boxes) = demo_app();
    app.focus = Pane::Tasks;

    assert!(matches!(press(&mut app, KeyCode::Char('m')), Effect::None));
    assert!(matches!(app.dialog, Some(Dialog::Move(_))));
}

#[test]
fn edit_opens_the_task_form() {
    let (mut app, _boxes) = demo_app();
    app.focus = Pane::Tasks;

    assert!(matches!(press(&mut app, KeyCode::Char('e')), Effect::None));
    assert!(app.form.is_some());
}

#[test]
fn a_new_list_starts_from_the_lists_pane() {
    let (mut app, _boxes) = demo_app();
    app.focus = Pane::Lists;

    assert!(matches!(press(&mut app, KeyCode::Char('a')), Effect::None));
    assert!(matches!(app.dialog, Some(Dialog::Name(_))));
}

#[test]
fn a_built_in_list_cannot_be_renamed() {
    let (mut app, _boxes) = demo_app();
    app.focus = Pane::Lists;
    let index = app
        .rows
        .iter()
        .position(|row| match row {
            tuilist::tui::app::ListRow::List { list, .. } => list.well_known.is_some(),
            _ => false,
        })
        .unwrap();
    app.select_list(index);

    assert!(matches!(press(&mut app, KeyCode::F(2)), Effect::None));
    assert!(matches!(app.status, Some(Status::Info(_))));
    assert!(app.dialog.is_none());
}

#[test]
fn space_marks_rows_and_escape_clears_them() {
    let (mut app, _boxes) = demo_app();
    app.focus = Pane::Tasks;

    press(&mut app, KeyCode::Char(' '));
    assert_eq!(app.marks.len(), 1);

    ctrl(&mut app, 'a');
    assert!(app.marks.len() > 1);

    press(&mut app, KeyCode::Esc);
    assert!(app.marks.is_empty());
}

#[test]
fn y_copies_the_title_and_c_hides_completed() {
    let (mut app, _boxes) = demo_app();
    app.focus = Pane::Tasks;

    let Effect::Copy(title) = press(&mut app, KeyCode::Char('y')) else {
        panic!("y should copy the title");
    };
    assert!(!title.is_empty());

    assert!(!app.completed_hidden);
    press(&mut app, KeyCode::Char('c'));
    assert!(app.completed_hidden);
}

#[test]
fn t_and_s_open_prompts_for_a_date_and_a_step() {
    let (mut app, _boxes) = demo_app();
    app.focus = Pane::Tasks;

    assert!(matches!(press(&mut app, KeyCode::Char('T')), Effect::None));
    assert!(matches!(app.dialog, Some(Dialog::Due(_))));
    app.dialog = None;

    assert!(matches!(press(&mut app, KeyCode::Char('s')), Effect::None));
    assert!(matches!(app.dialog, Some(Dialog::Name(_))));
}

#[test]
fn search_finds_tasks_in_both_accounts_and_escape_restores_them() {
    let (mut app, mut boxes) = demo_app();
    let all = app.tasks.len();

    press(&mut app, KeyCode::Char('/'));
    for c in "report".chars() {
        let effect = press(&mut app, KeyCode::Char(c));
        boxes.apply(&mut app, effect);
    }

    assert!(!app.tasks.is_empty());
    assert!(app.tasks.len() < all);
    assert!(
        app.tasks
            .iter()
            .all(|row| row.task.title.to_lowercase().contains("report"))
    );
    assert!(app.tasks.iter().any(|row| row.account == 0));
    assert!(app.tasks.iter().any(|row| row.account == 1));

    press(&mut app, KeyCode::Enter);
    assert_eq!(app.focus, Pane::Tasks);
    assert!(app.search.as_ref().is_some_and(|search| !search.typing()));

    let effect = press(&mut app, KeyCode::Esc);
    boxes.apply(&mut app, effect);
    assert!(app.search.is_none());
    assert_eq!(app.tasks.len(), all);
}
