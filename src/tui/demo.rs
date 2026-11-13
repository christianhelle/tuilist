//! The sample accounts behind `todo --demo`.

use jiff::{Span, Timestamp, Zoned};

use crate::{
    core::fake::FakeBackend,
    model::{Importance, ListId, Status, Step, StepId, Task, TaskId, TaskList, WellKnown},
};

struct Spec {
    id: &'static str,
    list: &'static str,
    title: &'static str,
    due: Option<i64>,
    starred: bool,
    completed: bool,
    recurring: bool,
    notes: &'static str,
}

const PERSONAL: &[Spec] = &[
    spec(
        "p-milk",
        "p-groceries",
        "Buy milk",
        Some(0),
        true,
        false,
        false,
        "Semi-skimmed",
    ),
    spec(
        "p-coffee",
        "p-groceries",
        "Coffee beans",
        Some(2),
        false,
        false,
        false,
        "The dark roast",
    ),
    spec(
        "p-bread",
        "p-groceries",
        "Bread",
        None,
        false,
        false,
        true,
        "",
    ),
    spec(
        "p-eggs",
        "p-groceries",
        "Eggs",
        None,
        false,
        true,
        false,
        "",
    ),
    spec(
        "p-butter",
        "p-groceries",
        "Butter",
        None,
        false,
        true,
        false,
        "",
    ),
    spec(
        "p-oat",
        "p-groceries",
        "Oat milk",
        Some(6),
        false,
        false,
        false,
        "",
    ),
    spec(
        "p-report",
        "p-work",
        "Send the status report",
        Some(0),
        true,
        false,
        false,
        "Include the store numbers",
    ),
    spec(
        "p-review",
        "p-work",
        "Review the pull request",
        Some(-1),
        false,
        false,
        false,
        "The engine and the sync loop",
    ),
    spec(
        "p-sprint",
        "p-work",
        "Plan the next sprint",
        Some(3),
        false,
        false,
        false,
        "",
    ),
    spec(
        "p-dune",
        "p-reading",
        "Read Dune",
        None,
        false,
        false,
        false,
        "The first hundred pages",
    ),
    spec(
        "p-articles",
        "p-reading",
        "Read the saved articles",
        Some(10),
        false,
        false,
        false,
        "",
    ),
    spec(
        "p-boxes",
        "p-shared",
        "Pack the boxes",
        Some(1),
        false,
        false,
        false,
        "Ask Mary for tape",
    ),
    spec(
        "p-plants",
        "p-tasks",
        "Water the plants",
        None,
        true,
        false,
        false,
        "",
    ),
    spec(
        "p-dentist",
        "p-tasks",
        "Call the dentist",
        Some(0),
        false,
        false,
        false,
        "",
    ),
    spec(
        "p-passport",
        "p-tasks",
        "Renew the passport",
        Some(30),
        false,
        false,
        false,
        "The form is in the drawer",
    ),
    spec(
        "p-rent",
        "p-tasks",
        "Pay the rent",
        Some(5),
        false,
        false,
        true,
        "",
    ),
    spec(
        "p-flights",
        "p-tasks",
        "Book the flights",
        Some(5),
        false,
        false,
        false,
        "",
    ),
    spec(
        "p-parcel",
        "p-tasks",
        "Return the parcel",
        Some(-2),
        false,
        false,
        false,
        "",
    ),
    spec(
        "p-bank",
        "p-flagged",
        "Email from the bank",
        None,
        true,
        false,
        false,
        "The statement is ready",
    ),
    spec("p-old", "p-tasks", "Old task", None, false, true, false, ""),
];

const WORK: &[Spec] = &[
    spec(
        "w-invoices",
        "w-tasks",
        "Send the invoices",
        Some(0),
        true,
        false,
        false,
        "",
    ),
    spec(
        "w-quarterly",
        "w-tasks",
        "Write the quarterly report",
        Some(2),
        false,
        false,
        false,
        "Numbers from finance",
    ),
    spec(
        "w-standup",
        "w-tasks",
        "Prepare the standup",
        None,
        false,
        false,
        true,
        "",
    ),
    spec(
        "w-laptop",
        "w-tasks",
        "Return the old laptop",
        Some(-3),
        false,
        false,
        false,
        "",
    ),
    spec(
        "w-expenses",
        "w-tasks",
        "File the expenses",
        Some(1),
        false,
        false,
        false,
        "",
    ),
    spec(
        "w-1on1",
        "w-tasks",
        "Book the one-to-one",
        Some(4),
        false,
        false,
        false,
        "",
    ),
    spec(
        "w-holiday",
        "w-tasks",
        "Request the holiday",
        Some(12),
        false,
        false,
        false,
        "Two weeks in June",
    ),
    spec(
        "w-done",
        "w-tasks",
        "Archive the old notes",
        None,
        false,
        true,
        false,
        "",
    ),
    spec(
        "w-spec",
        "w-projects",
        "Review the task specification",
        Some(0),
        true,
        false,
        false,
        "Sections 5 and 6",
    ),
    spec(
        "w-prototype",
        "w-projects",
        "Build the prototype",
        Some(7),
        false,
        false,
        false,
        "The lists, tasks and details first",
    ),
    spec(
        "w-demo",
        "w-projects",
        "Record the demo",
        Some(9),
        false,
        false,
        false,
        "",
    ),
    spec(
        "w-api",
        "w-projects",
        "Check the delta query",
        Some(-1),
        false,
        false,
        false,
        "Expired links must resync",
    ),
    spec(
        "w-book",
        "w-projects",
        "Order the design book",
        None,
        false,
        false,
        false,
        "",
    ),
    spec(
        "w-flagged",
        "w-flagged",
        "Email from the supplier",
        None,
        true,
        false,
        false,
        "The order shipped",
    ),
    spec(
        "w-audit",
        "w-tasks",
        "Reply to the auditor",
        Some(3),
        false,
        false,
        false,
        "",
    ),
    spec(
        "w-budget",
        "w-tasks",
        "Update the budget",
        Some(18),
        false,
        false,
        false,
        "",
    ),
    spec(
        "w-migration",
        "w-tasks",
        "Plan the migration",
        Some(2),
        true,
        false,
        false,
        "",
    ),
    spec(
        "w-lunch",
        "w-tasks",
        "Book the team lunch",
        Some(6),
        false,
        false,
        false,
        "The Italian place",
    ),
    spec("w-old", "w-tasks", "Old task", None, false, true, false, ""),
    spec(
        "w-contract",
        "w-tasks",
        "Renew the contract",
        None,
        false,
        false,
        true,
        "",
    ),
];

#[allow(clippy::too_many_arguments)]
const fn spec(
    id: &'static str,
    list: &'static str,
    title: &'static str,
    due: Option<i64>,
    starred: bool,
    completed: bool,
    recurring: bool,
    notes: &'static str,
) -> Spec {
    Spec {
        id,
        list,
        title,
        due,
        starred,
        completed,
        recurring,
        notes,
    }
}

/// Each demo account's address and the lists and tasks it has.
pub fn accounts() -> Vec<(String, FakeBackend)> {
    vec![
        ("ada@outlook.com".to_owned(), personal()),
        ("ada@work.example".to_owned(), work()),
    ]
}

fn personal() -> FakeBackend {
    let now = Timestamp::now().as_second();
    let today = Zoned::now().date();
    let lists = vec![
        well_known("p-tasks", "Tasks", WellKnown::DefaultList),
        list("p-groceries", "Groceries"),
        list("p-work", "Work"),
        list("p-reading", "Reading"),
        shared("p-shared", "Shared"),
        well_known("p-flagged", "Flagged email", WellKnown::FlaggedEmails),
    ];
    let steps = steps(&[
        ("p-milk", "Check the fridge", true),
        ("p-milk", "Semi-skimmed", false),
        ("p-coffee", "Grind the beans", true),
        ("p-coffee", "Fill the tin", false),
        ("p-coffee", "Order more", false),
        ("p-sprint", "Pick the stories", true),
        ("p-sprint", "Check the dates", true),
        ("p-sprint", "Tell the team", false),
        ("p-boxes", "Find the tape", false),
    ]);
    let tasks = build(PERSONAL, now, today, &steps);
    FakeBackend::new()
        .with_lists(&lists)
        .with_tasks(&tasks)
        .with_steps(&steps)
}

fn work() -> FakeBackend {
    let now = Timestamp::now().as_second();
    let today = Zoned::now().date();
    let lists = vec![
        well_known("w-tasks", "Tasks", WellKnown::DefaultList),
        list("w-projects", "Projects"),
        well_known("w-flagged", "Flagged email", WellKnown::FlaggedEmails),
    ];
    let steps = steps(&[
        ("w-spec", "Read section 5", true),
        ("w-spec", "Read section 6", false),
        ("w-quarterly", "Collect the numbers", true),
        ("w-quarterly", "Write the summary", true),
        ("w-quarterly", "Ask finance", true),
        ("w-quarterly", "Send it", false),
        ("w-prototype", "Lists", true),
        ("w-prototype", "Tasks", false),
        ("w-prototype", "Details", false),
        ("w-prototype", "Dialogs", false),
    ]);
    let tasks = build(WORK, now, today, &steps);
    FakeBackend::new()
        .with_lists(&lists)
        .with_tasks(&tasks)
        .with_steps(&steps)
}

fn list(id: &'static str, name: &'static str) -> TaskList {
    TaskList {
        id: ListId(id.to_owned()),
        name: name.to_owned(),
        well_known: None,
        is_owner: true,
        is_shared: false,
        open: 0,
    }
}

fn shared(id: &'static str, name: &'static str) -> TaskList {
    TaskList {
        is_shared: true,
        ..list(id, name)
    }
}

fn well_known(id: &'static str, name: &'static str, kind: WellKnown) -> TaskList {
    TaskList {
        well_known: Some(kind),
        ..list(id, name)
    }
}

fn steps(specs: &[(&'static str, &'static str, bool)]) -> Vec<Step> {
    specs
        .iter()
        .enumerate()
        .map(|(index, (task, name, checked))| Step {
            id: StepId(format!("{task}-s{index}")),
            task: TaskId((*task).to_owned()),
            name: (*name).to_owned(),
            checked: *checked,
            created_at: Timestamp::from_second(1_700_000_000 + index as i64).unwrap(),
        })
        .collect()
}

fn build(specs: &[Spec], now: i64, today: jiff::civil::Date, steps: &[Step]) -> Vec<Task> {
    specs
        .iter()
        .enumerate()
        .map(|(index, spec)| {
            let created = now - (index as i64 + 1) * 3_600;
            let due = spec
                .due
                .and_then(|days| today.checked_add(Span::new().days(days)).ok());
            let own: Vec<&Step> = steps.iter().filter(|step| step.task.0 == spec.id).collect();
            Task {
                id: TaskId(spec.id.to_owned()),
                list: ListId(spec.list.to_owned()),
                title: spec.title.to_owned(),
                status: if spec.completed {
                    Status::Completed
                } else {
                    Status::NotStarted
                },
                importance: if spec.starred {
                    Importance::High
                } else {
                    Importance::Normal
                },
                due,
                reminder: None,
                completed_at: spec
                    .completed
                    .then(|| Timestamp::from_second(created + 3_600).unwrap()),
                created_at: Timestamp::from_second(created).unwrap(),
                modified_at: Timestamp::from_second(created).unwrap(),
                notes: spec.notes.to_owned(),
                notes_html: false,
                recurring: spec.recurring,
                has_attachments: false,
                steps_done: u32::try_from(own.iter().filter(|step| step.checked).count())
                    .unwrap_or(0),
                steps_total: u32::try_from(own.len()).unwrap_or(0),
                steps_loaded: false,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_demo_has_two_accounts_and_every_state() {
        let accounts = accounts();
        assert_eq!(accounts.len(), 2);

        let tasks = accounts[0].1.tasks();
        assert!(tasks.iter().any(|task| task.importance.is_starred()));
        assert!(tasks.iter().any(|task| task.status == Status::Completed));
        assert!(tasks.iter().any(|task| task.due.is_some()));
        assert!(tasks.iter().any(|task| task.recurring));
        assert!(tasks.iter().any(|task| !task.notes.is_empty()));
    }

    #[test]
    fn the_demo_has_enough_tasks_for_a_screenful() {
        let total: usize = accounts()
            .iter()
            .map(|(_, backend)| backend.tasks().len())
            .sum();

        assert!(total >= 40, "only {total} tasks");
    }

    #[test]
    fn ids_are_unique_across_accounts() {
        let mut ids: Vec<String> = accounts()
            .iter()
            .flat_map(|(_, backend)| backend.tasks())
            .map(|task| task.id.0)
            .collect();
        let count = ids.len();
        ids.sort();
        ids.dedup();

        assert_eq!(ids.len(), count);
    }
}
