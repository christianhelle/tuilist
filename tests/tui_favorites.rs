use tuilist::{
    model::{Action, ListId, TaskList},
    tui::{
        app::{AppState, ListRow, Pane, Smart, Status},
        session::Session,
    },
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
            vec![list("projects", "Projects"), list("work-tasks", "Tasks")],
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
        Some(ListRow::List { list, .. }) if list.id == ListId("work".to_owned())
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

fn synced(open: u32) -> Vec<(String, Vec<TaskList>)> {
    vec![
        (
            "me@outlook.com".to_owned(),
            vec![
                TaskList {
                    open,
                    ..list("tasks", "Tasks")
                },
                list("work", "Work"),
            ],
        ),
        (
            "me@work.example".to_owned(),
            vec![list("projects", "Projects"), list("work-tasks", "Tasks")],
        ),
    ]
}

fn favorites(app: &AppState) -> usize {
    app.rows
        .iter()
        .take_while(|row| matches!(row, ListRow::Favorite { .. }))
        .count()
}

#[test]
fn a_selected_favorite_stays_selected_after_a_sync() {
    let mut app = app();
    select(&mut app, "tasks");
    app.update(Action::ToggleStar);
    select_favorite(&mut app, "tasks");

    app.set_accounts(synced(7));

    assert!(matches!(
        app.rows.get(app.selected_list),
        Some(ListRow::Favorite { list, .. }) if list.id == ListId("tasks".to_owned())
    ));
    assert_eq!(app.selected_list().unwrap().1.open, 7);
}

#[test]
fn a_selected_list_stays_an_ordinary_row_when_its_favorite_is_above_it() {
    let mut app = app();
    select(&mut app, "tasks");
    app.update(Action::ToggleStar);
    select(&mut app, "tasks");

    app.set_accounts(synced(0));

    assert!(matches!(
        app.rows.get(app.selected_list),
        Some(ListRow::List { list, .. }) if list.id == ListId("tasks".to_owned())
    ));
}

#[test]
fn a_favorite_whose_list_is_missing_waits_for_it() {
    let mut app = app();
    select(&mut app, "work");
    app.update(Action::ToggleStar);

    app.set_accounts(vec![
        ("me@outlook.com".to_owned(), vec![list("tasks", "Tasks")]),
        (
            "me@work.example".to_owned(),
            vec![list("projects", "Projects"), list("work-tasks", "Tasks")],
        ),
    ]);
    assert_eq!(favorites(&app), 0);
    assert_eq!(app.favorites, [ListId("work".to_owned())]);

    app.set_accounts(synced(0));

    assert_eq!(favorites(&app), 1);
    let ListRow::Favorite { list, .. } = &app.rows[0] else {
        panic!("the favorite comes back at the top");
    };
    assert_eq!(list.id, ListId("work".to_owned()));
}

#[test]
fn picking_a_favorite_shows_its_list() {
    let mut app = app();
    select(&mut app, "work");
    app.update(Action::ToggleStar);

    assert_eq!(app.selected_list_name(), "Work");
    assert!(matches!(
        app.view,
        tuilist::tui::app::View::List { account: 0, ref id }
            if id == &ListId("work".to_owned())
    ));
}

#[test]
fn the_session_brings_back_the_favorites_and_the_selected_one() {
    let mut started = app();
    select(&mut started, "tasks");
    started.update(Action::ToggleStar);
    select(&mut started, "work");
    started.update(Action::ToggleStar);
    select_favorite(&mut started, "tasks");

    let session = Session::of(&started);
    let mut restarted = app();
    session.apply(&mut restarted);

    assert_eq!(
        restarted.favorites,
        [ListId("tasks".to_owned()), ListId("work".to_owned())]
    );
    assert!(matches!(
        restarted.rows.get(restarted.selected_list),
        Some(ListRow::Favorite { list, .. }) if list.id == ListId("tasks".to_owned())
    ));
}

#[test]
fn the_session_does_not_turn_an_ordinary_list_pick_into_a_favorite_pick() {
    let mut started = app();
    select(&mut started, "tasks");
    started.update(Action::ToggleStar);
    select(&mut started, "tasks");

    let session = Session::of(&started);
    let mut restarted = app();
    session.apply(&mut restarted);

    assert!(matches!(
        restarted.rows.get(restarted.selected_list),
        Some(ListRow::List { list, .. }) if list.id == ListId("tasks".to_owned())
    ));
}

#[test]
fn favorites_with_the_same_name_say_which_account_they_are_from() {
    let mut app = app();
    for id in ["tasks", "work-tasks", "projects", "work"] {
        select(&mut app, id);
        app.update(Action::ToggleStar);
    }

    assert_eq!(
        app.favorite_label(0, &app.accounts[0].lists[0]),
        "Tasks - me@outlook.com"
    );
    assert_eq!(
        app.favorite_label(1, &app.accounts[1].lists[1]),
        "Tasks - me@work.example"
    );
    assert_eq!(app.favorite_label(1, &app.accounts[1].lists[0]), "Projects");
    assert_eq!(app.favorite_label(0, &app.accounts[0].lists[1]), "Work");
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
