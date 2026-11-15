use jiff::{Timestamp, civil, tz::TimeZone};
use reqwest::StatusCode;
use serde_json::{Map, Value, json};

use crate::{
    auth::TokenSource,
    graph::{
        GraphClient, GraphError,
        dto::{self, DateTimeTimeZone, Page, TodoList, TodoTask},
    },
    model::{
        BackendError, DeltaPage, Importance, ListId, Status, Step, StepId, Task, TaskDraft, TaskId,
        TaskList, TodoBackend, WellKnown,
    },
};

/// A Microsoft To Do account, over Graph.
pub struct GraphBackend<T> {
    client: GraphClient<T>,
    time_zone: TimeZone,
}

impl<T: TokenSource> GraphBackend<T> {
    pub fn new(client: GraphClient<T>) -> Self {
        Self {
            client,
            time_zone: named(TimeZone::system()),
        }
    }

    /// Reads and writes due dates and reminders in `time_zone` rather than
    /// the system's.
    pub fn with_time_zone(mut self, time_zone: TimeZone) -> Self {
        self.time_zone = named(time_zone);
        self
    }

    fn zone_name(&self) -> &str {
        self.time_zone.iana_name().unwrap_or("UTC")
    }

    fn list(&self, list: dto::TodoList) -> TaskList {
        TaskList {
            id: ListId(list.id),
            name: list.display_name,
            well_known: list
                .wellknown_list_name
                .as_deref()
                .and_then(WellKnown::from_graph),
            is_owner: list.is_owner.unwrap_or(true),
            is_shared: list.is_shared.unwrap_or(false),
            open: 0,
        }
    }

    fn task(&self, task: dto::TodoTask, list: &ListId) -> Task {
        let notes = task.body.unwrap_or(dto::ItemBody {
            content_type: String::new(),
            content: String::new(),
        });
        let notes_html = notes.content_type.eq_ignore_ascii_case("html");
        let notes_text = if notes_html {
            html2text::config::plain_no_decorate()
                .string_from_read(notes.content.as_bytes(), 1000)
                .map(|text| text.trim_end().to_owned())
                .unwrap_or_else(|_| notes.content.clone())
        } else {
            notes.content
        };
        Task {
            id: TaskId(task.id),
            list: list.clone(),
            title: task.title.unwrap_or_default(),
            status: task
                .status
                .as_deref()
                .map(Status::from_graph)
                .unwrap_or(Status::NotStarted),
            importance: task
                .importance
                .as_deref()
                .map(Importance::from_graph)
                .unwrap_or(Importance::Normal),
            due: task.due_date_time.and_then(|due| self.due_date(due)),
            reminder: task
                .is_reminder_on
                .unwrap_or(false)
                .then(|| task.reminder_date_time.and_then(|t| self.instant(t)))
                .flatten(),
            completed_at: task.completed_date_time.and_then(|t| self.instant(t)),
            created_at: task.created_date_time.unwrap_or(Timestamp::UNIX_EPOCH),
            modified_at: task
                .last_modified_date_time
                .unwrap_or(Timestamp::UNIX_EPOCH),
            notes: notes_text,
            notes_html,
            recurring: task.recurrence.is_some(),
            has_attachments: task.has_attachments.unwrap_or(false),
            steps_done: 0,
            steps_total: 0,
            steps_loaded: false,
        }
    }

    fn step(&self, step: dto::ChecklistItem, task: &TaskId) -> Step {
        Step {
            id: StepId(step.id),
            task: task.clone(),
            name: step.display_name,
            checked: step.is_checked,
            created_at: step.created_date_time.unwrap_or(Timestamp::UNIX_EPOCH),
        }
    }

    /// A wall-clock time from Graph, in our time zone, as an instant.
    fn instant(&self, time: DateTimeTimeZone) -> Option<Timestamp> {
        let wall = time.date_time.parse::<civil::DateTime>().ok()?;
        let zone = time
            .time_zone
            .as_deref()
            .and_then(|name| TimeZone::get(name).ok())
            .unwrap_or(TimeZone::UTC);
        let zoned = wall.to_zoned(zone).ok()?;
        Some(
            zoned
                .timestamp()
                .to_zoned(self.time_zone.clone())
                .timestamp(),
        )
    }

    /// A due date from Graph, as the day it is in our time zone.
    fn due_date(&self, due: DateTimeTimeZone) -> Option<jiff::civil::Date> {
        self.instant(due)
            .map(|instant| instant.to_zoned(self.time_zone.clone()).date())
    }

    /// A time as Graph takes it: the wall clock in our time zone, by name.
    fn wall_clock(&self, time: Timestamp) -> Value {
        let local = time.to_zoned(self.time_zone.clone());
        json!({
            "dateTime": local.strftime("%Y-%m-%dT%H:%M:%S").to_string(),
            "timeZone": self.zone_name(),
        })
    }

    fn due_json(&self, date: civil::Date) -> Value {
        json!({
            "dateTime": format!("{date}T00:00:00"),
            "timeZone": self.zone_name(),
        })
    }

    fn draft_json(&self, draft: &TaskDraft) -> Value {
        let mut body = Map::new();
        if let Some(title) = &draft.title {
            body.insert("title".to_owned(), title.clone().into());
        }
        if let Some(starred) = draft.starred {
            body.insert(
                "importance".to_owned(),
                if starred { "high" } else { "normal" }.into(),
            );
        }
        if let Some(completed) = draft.completed {
            body.insert(
                "status".to_owned(),
                if completed { "completed" } else { "notStarted" }.into(),
            );
        }
        if let Some(due) = &draft.due {
            body.insert(
                "dueDateTime".to_owned(),
                due.map_or(Value::Null, |date| self.due_json(date)),
            );
        }
        if let Some(reminder) = &draft.reminder {
            match reminder {
                Some(time) => {
                    body.insert("reminderDateTime".to_owned(), self.wall_clock(*time));
                    body.insert("isReminderOn".to_owned(), true.into());
                }
                None => {
                    body.insert("isReminderOn".to_owned(), false.into());
                }
            }
        }
        if let Some(notes) = &draft.notes {
            body.insert(
                "body".to_owned(),
                json!({ "content": notes, "contentType": "text" }),
            );
        }
        Value::Object(body)
    }

    async fn get<R: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        headers: &[(&str, &str)],
    ) -> Result<R, BackendError> {
        self.client.get(path, headers).await.map_err(backend_error)
    }

    fn list_path(list: &ListId) -> String {
        format!("/me/todo/lists/{}", list.0)
    }

    fn task_path(list: &ListId, task: &TaskId) -> String {
        format!("/me/todo/lists/{}/tasks/{}", list.0, task.0)
    }

    fn step_path(list: &ListId, task: &TaskId, step: &StepId) -> String {
        format!("{}/checklistItems/{}", Self::task_path(list, task), step.0)
    }
}

impl<T: TokenSource> TodoBackend for GraphBackend<T> {
    async fn lists_delta(
        &self,
        link: Option<&str>,
    ) -> Result<DeltaPage<TaskList, ListId>, BackendError> {
        let mut page = DeltaPage::default();
        let mut path = link.unwrap_or("/me/todo/lists/delta").to_owned();
        loop {
            let answer: Page<TodoList> = self.get(&path, &[]).await?;
            for item in answer.value {
                if item.removed.is_some() {
                    page.removed.push(ListId(item.id));
                } else {
                    page.upserted.push(self.list(item));
                }
            }
            match answer.next_link {
                Some(next) => path = next,
                None => {
                    page.delta_link = answer.delta_link;
                    break;
                }
            }
        }
        Ok(page)
    }

    async fn tasks_delta(
        &self,
        list: &ListId,
        link: Option<&str>,
    ) -> Result<DeltaPage<Task, TaskId>, BackendError> {
        let mut page = DeltaPage::default();
        let mut path = link
            .map(str::to_owned)
            .unwrap_or_else(|| format!("{}/tasks/delta", Self::list_path(list)));
        loop {
            let answer: Page<TodoTask> = self
                .get(&path, &[("Prefer", "odata.maxpagesize=50")])
                .await?;
            for item in answer.value {
                if item.removed.is_some() {
                    page.removed.push(TaskId(item.id));
                } else {
                    page.upserted.push(self.task(item, list));
                }
            }
            match answer.next_link {
                Some(next) => path = next,
                None => {
                    page.delta_link = answer.delta_link;
                    break;
                }
            }
        }
        Ok(page)
    }

    async fn create_list(&self, name: &str) -> Result<TaskList, BackendError> {
        let body = json!({ "displayName": name });
        let list: TodoList = self
            .client
            .post("/me/todo/lists", &body)
            .await
            .map_err(backend_error)?;
        Ok(self.list(list))
    }

    async fn rename_list(&self, list: &ListId, name: &str) -> Result<(), BackendError> {
        let body = json!({ "displayName": name });
        self.client
            .patch(&Self::list_path(list), &body)
            .await
            .map_err(backend_error)
    }

    async fn delete_list(&self, list: &ListId) -> Result<(), BackendError> {
        self.client
            .delete(&Self::list_path(list))
            .await
            .map_err(backend_error)
    }

    async fn create_task(&self, list: &ListId, draft: &TaskDraft) -> Result<Task, BackendError> {
        let path = format!("{}/tasks", Self::list_path(list));
        let task: TodoTask = self
            .client
            .post(&path, &self.draft_json(draft))
            .await
            .map_err(backend_error)?;
        Ok(self.task(task, list))
    }

    async fn update_task(
        &self,
        list: &ListId,
        task: &TaskId,
        draft: &TaskDraft,
    ) -> Result<Task, BackendError> {
        let answer: TodoTask = self
            .client
            .patch_json(&Self::task_path(list, task), &self.draft_json(draft))
            .await
            .map_err(backend_error)?;
        Ok(self.task(answer, list))
    }

    async fn delete_task(&self, list: &ListId, task: &TaskId) -> Result<(), BackendError> {
        self.client
            .delete(&Self::task_path(list, task))
            .await
            .map_err(backend_error)
    }

    async fn steps(&self, list: &ListId, task: &TaskId) -> Result<Vec<Step>, BackendError> {
        let mut steps = Vec::new();
        let mut path = format!("{}/checklistItems", Self::task_path(list, task));
        loop {
            let answer: Page<dto::ChecklistItem> = self.get(&path, &[]).await?;
            steps.extend(answer.value.into_iter().map(|s| self.step(s, task)));
            match answer.next_link {
                Some(next) => path = next,
                None => break,
            }
        }
        Ok(steps)
    }

    async fn create_step(
        &self,
        list: &ListId,
        task: &TaskId,
        name: &str,
    ) -> Result<Step, BackendError> {
        let path = format!("{}/checklistItems", Self::task_path(list, task));
        let body = json!({ "displayName": name });
        let step: dto::ChecklistItem = self
            .client
            .post(&path, &body)
            .await
            .map_err(backend_error)?;
        Ok(self.step(step, task))
    }

    async fn update_step(
        &self,
        list: &ListId,
        task: &TaskId,
        step: &StepId,
        name: Option<&str>,
        checked: Option<bool>,
    ) -> Result<(), BackendError> {
        let mut body = Map::new();
        if let Some(name) = name {
            body.insert("displayName".to_owned(), name.into());
        }
        if let Some(checked) = checked {
            body.insert("isChecked".to_owned(), checked.into());
        }
        self.client
            .patch(&Self::step_path(list, task, step), &Value::Object(body))
            .await
            .map_err(backend_error)
    }

    async fn delete_step(
        &self,
        list: &ListId,
        task: &TaskId,
        step: &StepId,
    ) -> Result<(), BackendError> {
        self.client
            .delete(&Self::step_path(list, task, step))
            .await
            .map_err(backend_error)
    }
}

fn backend_error(error: GraphError) -> BackendError {
    match &error {
        GraphError::Auth(crate::auth::AuthError::NotSignedIn) => BackendError::SignedOut,
        GraphError::Status { status, .. } if *status == StatusCode::GONE => {
            BackendError::DeltaExpired
        }
        GraphError::Status { status, .. } if *status == StatusCode::NOT_FOUND => {
            BackendError::NotFound
        }
        _ => BackendError::Other(error.to_string()),
    }
}

/// `time_zone` if Graph can be told its name, or else UTC.
fn named(time_zone: TimeZone) -> TimeZone {
    if time_zone.iana_name().is_some() {
        time_zone
    } else {
        TimeZone::UTC
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn backend() -> GraphBackend<NoToken> {
        GraphBackend::with_time_zone(
            GraphBackend::new(GraphClient::new("http://graph", NoToken)),
            TimeZone::get("UTC").unwrap(),
        )
    }

    struct NoToken;

    impl TokenSource for NoToken {
        async fn access_token(&self) -> Result<String, crate::auth::AuthError> {
            Ok("token".to_owned())
        }

        async fn invalidate(&self) {}
    }

    fn task(fields: Value) -> dto::TodoTask {
        serde_json::from_value(fields).unwrap()
    }

    #[test]
    fn statuses_and_importance_are_read_as_graph_names() {
        let backend = backend();
        let list = ListId("list".to_owned());
        let task = backend.task(
            task(json!({
                "id": "1",
                "title": "Milk",
                "status": "waitingOnOthers",
                "importance": "high"
            })),
            &list,
        );

        assert_eq!(task.status, Status::WaitingOnOthers);
        assert_eq!(task.importance, Importance::High);
    }

    #[test]
    fn a_due_date_keeps_its_day_in_our_zone() {
        let backend = backend();
        let list = ListId("list".to_owned());
        let task = backend.task(
            task(json!({
                "id": "1",
                "dueDateTime": {
                    "dateTime": "2026-11-20T00:00:00.0000000",
                    "timeZone": "UTC"
                }
            })),
            &list,
        );

        assert_eq!(task.due.as_ref().unwrap().to_string(), "2026-11-20");
    }

    #[test]
    fn a_reminder_only_counts_when_it_is_on() {
        let backend = backend();
        let list = ListId("list".to_owned());
        let off = backend.task(
            task(json!({
                "id": "1",
                "isReminderOn": false,
                "reminderDateTime": {
                    "dateTime": "2026-11-20T09:00:00",
                    "timeZone": "UTC"
                }
            })),
            &list,
        );

        assert!(off.reminder.is_none());
    }

    #[test]
    fn html_notes_are_shown_as_text_but_remembered_as_html() {
        let backend = backend();
        let list = ListId("list".to_owned());
        let task = backend.task(
            task(json!({
                "id": "1",
                "body": {
                    "contentType": "html",
                    "content": "<p>Buy <b>milk</b></p>"
                }
            })),
            &list,
        );

        assert_eq!(task.notes, "Buy milk");
        assert!(task.notes_html);
    }

    #[test]
    fn writes_carry_only_the_fields_the_draft_sets() {
        let body = backend().draft_json(&TaskDraft {
            starred: Some(true),
            ..TaskDraft::default()
        });

        assert_eq!(body, json!({ "importance": "high" }));
    }
}
