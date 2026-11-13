use ratatui::{Terminal, backend::TestBackend};
use tuilist::{
    core::{sync_lists, sync_tasks},
    store::Store,
    tui::{app::AppState, demo, taskboxes::TaskBoxes, ui},
};

fn demo_stores() -> Vec<(String, Store)> {
    futures::executor::block_on(async {
        let mut accounts = Vec::new();
        for (address, backend) in demo::accounts() {
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
    let mut app = AppState::default();
    let mut boxes = TaskBoxes::new(demo_stores());
    boxes.load(&mut app).unwrap();

    let screen = screen(&mut app);

    assert!(screen.contains("Groceries"), "{screen}");
    assert!(screen.contains("Buy milk"), "{screen}");
    assert!(screen.contains("Today"), "{screen}");
    assert!(screen.contains("ada@outlook.com"), "{screen}");
    assert!(screen.contains("? help"), "{screen}");
}

#[test]
fn the_help_dialog_covers_the_screen() {
    let mut app = AppState::default();
    let mut boxes = TaskBoxes::new(demo_stores());
    boxes.load(&mut app).unwrap();
    app.dialog = Some(tuilist::tui::dialog::Dialog::Help);

    let screen = screen(&mut app);

    assert!(screen.contains("Keys"), "{screen}");
    assert!(screen.contains("next or previous pane"), "{screen}");
}
