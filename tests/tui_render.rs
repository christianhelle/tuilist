use jiff::Zoned;
use ratatui::{Terminal, backend::TestBackend};
use tuilist::{
    core::{sync_lists, sync_tasks},
    model::ListId,
    store::Store,
    tui::{
        app::{AppState, Pane, View},
        demo,
        taskboxes::TaskBoxes,
        ui,
    },
};

fn demo_now() -> Zoned {
    "2026-11-13T09:00:00[UTC]".parse().unwrap()
}

fn demo_stores() -> Vec<(String, Store)> {
    futures::executor::block_on(async {
        let mut accounts = Vec::new();
        for (address, backend) in demo::accounts_at(demo_now()) {
            let mut store = Store::open_in_memory().unwrap();
            sync_lists(&backend, &mut store).await.unwrap();
            let lists = store.lists().unwrap();
            for list in &lists {
                sync_tasks(&backend, &mut store, &list.id).await.unwrap();
            }
            for list in &lists {
                for task in store.tasks_in(&list.id).unwrap() {
                    let steps = backend.steps_of(&task.id);
                    if !steps.is_empty() {
                        store.put_steps(&task.id, &steps).unwrap();
                    }
                }
            }
            accounts.push((address, store));
        }
        accounts
    })
}

fn demo_app() -> (AppState, TaskBoxes) {
    let mut app = AppState {
        clock: Some(demo_now()),
        ..AppState::default()
    };
    let mut boxes = TaskBoxes::new(demo_stores());
    boxes.load(&mut app).unwrap();
    (app, boxes)
}

fn screen(app: &mut AppState) -> String {
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal.draw(|frame| ui::draw(frame, app)).unwrap();
    let buffer = terminal.backend().buffer();
    let width = usize::from(buffer.area.width);
    buffer
        .content()
        .chunks(width)
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_demo_paints_lists_tasks_and_a_status_line() {
    let (mut app, _boxes) = demo_app();

    let screen = screen(&mut app);

    assert!(screen.contains("Groceries"), "{screen}");
    assert!(screen.contains("Buy milk"), "{screen}");
    assert!(screen.contains("Today"), "{screen}");
    assert!(screen.contains("ada@outlook.com"), "{screen}");
    assert!(screen.contains("? help"), "{screen}");
}

#[test]
fn the_help_dialog_covers_the_screen() {
    let (mut app, _boxes) = demo_app();
    app.dialog = Some(tuilist::tui::dialog::Dialog::Help);

    let screen = screen(&mut app);

    assert!(screen.contains("Keys"), "{screen}");
    assert!(screen.contains("next or previous pane"), "{screen}");
}

#[test]
fn the_lists_pane_is_the_default_focus() {
    let (mut app, _boxes) = demo_app();

    insta::assert_snapshot!(screen(&mut app));
}

#[test]
fn the_tasks_pane_can_take_focus() {
    let (mut app, _boxes) = demo_app();
    app.focus = Pane::Tasks;

    insta::assert_snapshot!(screen(&mut app));
}

#[test]
fn the_details_pane_can_take_focus() {
    let (mut app, _boxes) = demo_app();
    app.focus = Pane::Details;

    insta::assert_snapshot!(screen(&mut app));
}

#[test]
fn an_empty_list_says_there_is_nothing_in_it() {
    let (mut app, mut boxes) = demo_app();
    app.view = View::List {
        account: 0,
        id: ListId("missing".to_owned()),
    };
    boxes.load(&mut app).unwrap();
    app.focus = Pane::Tasks;

    insta::assert_snapshot!(screen(&mut app));
}

#[test]
fn a_folded_group_hides_its_tasks() {
    let (mut app, _boxes) = demo_app();
    app.focus = Pane::Tasks;
    app.collapsed_groups.insert("plain".to_owned());

    insta::assert_snapshot!(screen(&mut app));
}

#[test]
fn a_zoomed_tasks_pane_fills_the_screen() {
    let (mut app, _boxes) = demo_app();
    app.focus = Pane::Tasks;
    app.zoomed = true;

    insta::assert_snapshot!(screen(&mut app));
}

#[test]
fn details_can_sit_below_the_tasks() {
    let (mut app, _boxes) = demo_app();
    app.focus = Pane::Tasks;
    app.details_below = true;

    insta::assert_snapshot!(screen(&mut app));
}
