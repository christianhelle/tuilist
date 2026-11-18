use tuilist::{
    model::{Action, ListId, TaskList},
    tui::app::{AppState, ListRow, Pane, Smart, Status},
};

fn list(id: &str, name: &str) -> TaskList {
    TaskList {
        id: ListId(id.to_owned()),
        name: name.to_owned(),
        well_known: None,
        is_owner: true,
        is_shared: false,
        open: 0,
    }
}

fn app() -> AppState {
    let mut app = AppState::default();
    app.set_accounts(vec![
        (
            "me@outlook.com".to_owned(),
            vec![list("tasks", "Tasks"), list("work", "Work")],
        ),
        (
            "me@work.example".to_owned(),
            vec![list("projects", "Projects")],
        ),
    ]);
    app
}

fn select(app: &mut AppState, id: &str) {
    let index = app
        .rows
        .iter()
        .position(|row| match row {
            ListRow::List { list, .. } => list.id.0 == id,
            _ => false,
        })
        .unwrap();
    app.select_list(index);
}

#[test]
fn star_adds_the_list_to_the_favorites_at_the_top() {
    let mut app = app();
    select(&mut app, "work");

    assert!(matches!(
        app.update(Action::ToggleStar),
        tuilist::tui::app::Effect::None
    ));

    assert_eq!(app.favorites, [ListId("work".to_owned())]);
    let ListRow::Favorite { list, .. } = &app.rows[0] else {
        panic!("the favorite comes first");
    };
    assert_eq!(list.id, ListId("work".to_owned()));
    assert!(matches!(
        app.rows.get(app.selected_list),
        Some(ListRow::Favorite { list, .. }) if list.id == ListId("work".to_owned())
    ));
}

fn select_favorite(app: &mut AppState, id: &str) {
    let index = app
        .rows
        .iter()
        .position(|row| match row {
            ListRow::Favorite { list, .. } => list.id.0 == id,
            _ => false,
        })
        .unwrap();
    app.select_list(index);
}

#[test]
fn star_again_takes_the_list_out_from_either_row() {
    let mut app = app();
    select(&mut app, "work");
    app.update(Action::ToggleStar);
    select_favorite(&mut app, "work");

    app.update(Action::ToggleStar);

    assert!(app.favorites.is_empty());
    assert!(matches!(
        app.rows.get(app.selected_list),
        Some(ListRow::List { list, .. }) if list.id == ListId("work".to_owned())
    ));

    select(&mut app, "work");
    app.update(Action::ToggleStar);
    select(&mut app, "work");
    app.update(Action::ToggleStar);

    assert!(app.favorites.is_empty());
}

#[test]
fn star_does_nothing_on_a_smart_view_or_outside_the_lists_pane() {
    let mut app = app();
    let smart = app
        .rows
        .iter()
        .position(|row| matches!(row, ListRow::Smart(Smart::All)))
        .unwrap();
    app.select_list(smart);
    app.update(Action::ToggleStar);

    select(&mut app, "work");
    app.focus = Pane::Tasks;
    app.update(Action::ToggleStar);

    assert!(app.favorites.is_empty());
}

#[test]
fn the_status_line_says_when_a_list_joins_or_leaves_the_favorites() {
    let mut app = app();
    select(&mut app, "work");

    app.update(Action::ToggleStar);

    assert!(matches!(
        &app.status,
        Some(Status::Info(text)) if text == "Added Work to Favorites"
    ));

    app.update(Action::ToggleStar);

    assert!(matches!(
        &app.status,
        Some(Status::Info(text)) if text == "Removed Work from Favorites"
    ));
}
