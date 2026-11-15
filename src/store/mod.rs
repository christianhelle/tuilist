//! Local SQLite cache. The UI reads from it; only core writes to it.

use std::{collections::HashSet, sync::LazyLock};

use jiff::{Timestamp, civil::Date};
use rusqlite::{Connection, OptionalExtension, Row, types::Type};
use rusqlite_migration::{M, Migrations};

use crate::model::{
    Importance, ListId, Status, Step, StepId, Task, TaskDraft, TaskId, TaskList, WellKnown,
};

static MIGRATIONS: LazyLock<Migrations<'static>> = LazyLock::new(|| {
    Migrations::new(vec![M::up(include_str!(
        "migrations/001_lists_tasks_steps.sql"
    ))])
});

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Migration(#[from] rusqlite_migration::Error),
}

pub type Result<T> = std::result::Result<T, StoreError>;

pub struct Store {
    conn: Connection,
    path: Option<std::path::PathBuf>,
}

impl Store {
    /// Opens (creating if needed) the cache at `path`. Several handles may
    /// share one file: the UI reads while core writes.
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        // WAL lets readers proceed while the sync engine writes.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let mut store = Self::init(conn)?;
        store.path = Some(path.to_owned());
        Ok(store)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self> {
        conn.pragma_update(None, "foreign_keys", true)?;
        MIGRATIONS.to_latest(&mut conn)?;
        Ok(Self { conn, path: None })
    }

    /// Where this cache lives, when it lives on disk.
    pub fn path(&self) -> Option<&std::path::Path> {
        self.path.as_deref()
    }

    pub fn upsert_lists(&mut self, lists: &[TaskList]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO lists (id, name, well_known, is_owner, is_shared)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (id) DO UPDATE SET
                     name = excluded.name,
                     well_known = excluded.well_known,
                     is_owner = excluded.is_owner,
                     is_shared = excluded.is_shared",
            )?;
            for list in lists {
                stmt.execute((
                    &list.id.0,
                    &list.name,
                    list.well_known.map(WellKnown::as_str),
                    list.is_owner,
                    list.is_shared,
                ))?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Deletes every list not in `keep`, and the tasks in them.
    pub fn retain_lists(&mut self, keep: &[ListId]) -> Result<()> {
        let keep: HashSet<&str> = keep.iter().map(|id| id.0.as_str()).collect();
        let tx = self.conn.transaction()?;
        {
            let stale: Vec<String> = tx
                .prepare("SELECT id FROM lists")?
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?
                .into_iter()
                .filter(|id| !keep.contains(id.as_str()))
                .collect();
            let mut stmt = tx.prepare("DELETE FROM lists WHERE id = ?1")?;
            for id in stale {
                stmt.execute([id])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn remove_lists(&mut self, lists: &[ListId]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare("DELETE FROM lists WHERE id = ?1")?;
            for id in lists {
                stmt.execute([&id.0])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Every cached list, with the number of open tasks in each.
    pub fn lists(&self) -> Result<Vec<TaskList>> {
        let mut stmt = self.conn.prepare(
            "SELECT l.id, l.name, l.well_known, l.is_owner, l.is_shared,
                    (SELECT COUNT(*) FROM tasks t
                     WHERE t.list_id = l.id AND t.status != 'completed')
             FROM lists l",
        )?;
        let lists = stmt
            .query_map([], |row| {
                Ok(TaskList {
                    id: ListId(row.get(0)?),
                    name: row.get(1)?,
                    well_known: row
                        .get::<_, Option<String>>(2)?
                        .and_then(|value| WellKnown::from_graph(&value)),
                    is_owner: row.get(3)?,
                    is_shared: row.get(4)?,
                    open: row.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(lists)
    }

    pub fn upsert_tasks(&mut self, tasks: &[Task]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO tasks (
                     id, list_id, title, status, importance, due, reminder_at,
                     completed_at, created_at, modified_at, notes, notes_html,
                     recurring, has_attachments, steps_loaded)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
                 ON CONFLICT (id) DO UPDATE SET
                     list_id = excluded.list_id,
                     title = excluded.title,
                     status = excluded.status,
                     importance = excluded.importance,
                     due = excluded.due,
                     reminder_at = excluded.reminder_at,
                     completed_at = excluded.completed_at,
                     created_at = excluded.created_at,
                     modified_at = excluded.modified_at,
                     notes = excluded.notes,
                     notes_html = excluded.notes_html,
                     recurring = excluded.recurring,
                     has_attachments = excluded.has_attachments,
                     steps_loaded = CASE
                         WHEN tasks.modified_at = excluded.modified_at
                         THEN tasks.steps_loaded
                         ELSE 0 END",
            )?;
            for task in tasks {
                stmt.execute((
                    &task.id.0,
                    &task.list.0,
                    &task.title,
                    task.status.as_str(),
                    task.importance.as_str(),
                    task.due.map(|date| date.to_string()),
                    task.reminder.map(|at| at.as_millisecond()),
                    task.completed_at.map(|at| at.as_millisecond()),
                    task.created_at.as_millisecond(),
                    task.modified_at.as_millisecond(),
                    &task.notes,
                    task.notes_html,
                    task.recurring,
                    task.has_attachments,
                    task.steps_loaded,
                ))?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn remove_tasks(&mut self, tasks: &[TaskId]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare("DELETE FROM tasks WHERE id = ?1")?;
            for id in tasks {
                stmt.execute([&id.0])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn clear_tasks_in(&mut self, list: &ListId) -> Result<()> {
        self.conn
            .execute("DELETE FROM tasks WHERE list_id = ?1", [&list.0])?;
        Ok(())
    }

    pub fn tasks_in(&self, list: &ListId) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(&format!(
            "{TASK_SELECT} WHERE t.list_id = ?1 ORDER BY t.created_at DESC"
        ))?;
        let tasks = stmt
            .query_map([&list.0], task_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(tasks)
    }

    /// Open, starred tasks.
    pub fn important(&self) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(&format!(
            "{TASK_SELECT} WHERE t.status != 'completed' AND t.importance = 'high'
             ORDER BY t.created_at DESC"
        ))?;
        let tasks = stmt
            .query_map([], task_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(tasks)
    }

    /// Open tasks with a due date, soonest first.
    pub fn planned(&self) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(&format!(
            "{TASK_SELECT} WHERE t.status != 'completed' AND t.due IS NOT NULL
             ORDER BY t.due, t.created_at DESC"
        ))?;
        let tasks = stmt
            .query_map([], task_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(tasks)
    }

    pub fn all_open(&self) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(&format!(
            "{TASK_SELECT} WHERE t.status != 'completed' ORDER BY t.created_at DESC"
        ))?;
        let tasks = stmt
            .query_map([], task_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(tasks)
    }

    pub fn task(&self, id: &TaskId) -> Result<Option<Task>> {
        let mut stmt = self
            .conn
            .prepare(&format!("{TASK_SELECT} WHERE t.id = ?1"))?;
        Ok(stmt.query_row([&id.0], task_from_row).optional()?)
    }

    /// Applies a draft to a cached task, leaving the fields it does not carry
    /// and the task's sync state as they are.
    pub fn change_task(&mut self, id: &TaskId, draft: &TaskDraft) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            if let Some(title) = &draft.title {
                tx.execute(
                    "UPDATE tasks SET title = ?1 WHERE id = ?2",
                    rusqlite::params![title, &id.0],
                )?;
            }
            if let Some(starred) = draft.starred {
                let importance = if starred {
                    Importance::High
                } else {
                    Importance::Normal
                };
                tx.execute(
                    "UPDATE tasks SET importance = ?1 WHERE id = ?2",
                    rusqlite::params![importance.as_str(), &id.0],
                )?;
            }
            if let Some(completed) = draft.completed {
                let status = if completed {
                    Status::Completed
                } else {
                    Status::NotStarted
                };
                let completed_at = if completed {
                    Some(Timestamp::now().as_millisecond())
                } else {
                    None
                };
                tx.execute(
                    "UPDATE tasks SET status = ?1, completed_at = ?2 WHERE id = ?3",
                    rusqlite::params![status.as_str(), completed_at, &id.0],
                )?;
            }
            if let Some(due) = draft.due {
                tx.execute(
                    "UPDATE tasks SET due = ?1 WHERE id = ?2",
                    rusqlite::params![due.map(|date| date.to_string()), &id.0],
                )?;
            }
            if let Some(reminder) = draft.reminder {
                tx.execute(
                    "UPDATE tasks SET reminder_at = ?1 WHERE id = ?2",
                    rusqlite::params![reminder.map(|at| at.as_millisecond()), &id.0],
                )?;
            }
            if let Some(notes) = &draft.notes {
                tx.execute(
                    "UPDATE tasks SET notes = ?1, notes_html = 0 WHERE id = ?2",
                    rusqlite::params![notes, &id.0],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Replaces a task's cached steps with `steps`.
    pub fn put_steps(&mut self, task: &TaskId, steps: &[Step]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            tx.execute("DELETE FROM steps WHERE task_id = ?1", [&task.0])?;
            let mut stmt = tx.prepare(
                "INSERT INTO steps (id, task_id, name, checked, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for step in steps {
                stmt.execute((
                    &step.id.0,
                    &step.task.0,
                    &step.name,
                    step.checked,
                    step.created_at.as_millisecond(),
                ))?;
            }
            drop(stmt);
            tx.execute("UPDATE tasks SET steps_loaded = 1 WHERE id = ?1", [&task.0])?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn steps(&self, task: &TaskId) -> Result<Vec<Step>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task_id, name, checked, created_at
             FROM steps WHERE task_id = ?1 ORDER BY created_at, id",
        )?;
        let steps = stmt
            .query_map([&task.0], |row| {
                Ok(Step {
                    id: StepId(row.get(0)?),
                    task: TaskId(row.get(1)?),
                    name: row.get(2)?,
                    checked: row.get(3)?,
                    created_at: timestamp(row, 4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(steps)
    }

    pub fn upsert_step(&mut self, step: &Step) -> Result<()> {
        self.conn.execute(
            "INSERT INTO steps (id, task_id, name, checked, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (id) DO UPDATE SET
                 name = excluded.name,
                 checked = excluded.checked",
            (
                &step.id.0,
                &step.task.0,
                &step.name,
                step.checked,
                step.created_at.as_millisecond(),
            ),
        )?;
        Ok(())
    }

    pub fn remove_step(&mut self, step: &StepId) -> Result<()> {
        self.conn
            .execute("DELETE FROM steps WHERE id = ?1", [&step.0])?;
        Ok(())
    }

    /// Tasks whose title or notes contain `query`, ignoring case.
    pub fn search(&self, query: &str, limit: u32) -> Result<Vec<Task>> {
        let pattern = format!("%{}%", escape_like(query));
        let mut stmt = self.conn.prepare(&format!(
            "{TASK_SELECT}
             WHERE t.title LIKE ?1 ESCAPE '\\' OR t.notes LIKE ?1 ESCAPE '\\'
             ORDER BY t.status = 'completed', t.modified_at DESC
             LIMIT ?2"
        ))?;
        let tasks = stmt
            .query_map(rusqlite::params![pattern, limit], task_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(tasks)
    }

    pub fn delta_link(&self, list: &ListId) -> Result<Option<String>> {
        let link = self
            .conn
            .query_row(
                "SELECT delta_link FROM lists WHERE id = ?1",
                [&list.0],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        Ok(link)
    }

    pub fn set_delta_link(&mut self, list: &ListId, link: Option<&str>) -> Result<()> {
        self.conn.execute(
            "UPDATE lists SET delta_link = ?1 WHERE id = ?2",
            rusqlite::params![link, &list.0],
        )?;
        Ok(())
    }

    pub fn lists_delta_link(&self) -> Result<Option<String>> {
        let link = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'lists_delta_link'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(link)
    }

    pub fn set_lists_delta_link(&mut self, link: Option<&str>) -> Result<()> {
        match link {
            Some(link) => self.conn.execute(
                "INSERT INTO meta (key, value) VALUES ('lists_delta_link', ?1)
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                [link],
            )?,
            None => self
                .conn
                .execute("DELETE FROM meta WHERE key = 'lists_delta_link'", [])?,
        };
        Ok(())
    }
}

const TASK_SELECT: &str = "SELECT t.id, t.list_id, t.title, t.status, t.importance, t.due,
        t.reminder_at, t.completed_at, t.created_at, t.modified_at, t.notes,
        t.notes_html, t.recurring, t.has_attachments, t.steps_loaded,
        (SELECT COUNT(*) FROM steps s WHERE s.task_id = t.id AND s.checked = 1),
        (SELECT COUNT(*) FROM steps s WHERE s.task_id = t.id)
     FROM tasks t";

fn task_from_row(row: &Row) -> rusqlite::Result<Task> {
    Ok(Task {
        id: TaskId(row.get(0)?),
        list: ListId(row.get(1)?),
        title: row.get(2)?,
        status: Status::from_graph(&row.get::<_, String>(3)?),
        importance: Importance::from_graph(&row.get::<_, String>(4)?),
        due: row
            .get::<_, Option<String>>(5)?
            .and_then(|date| date.parse::<Date>().ok()),
        reminder: optional_timestamp(row, 6)?,
        completed_at: optional_timestamp(row, 7)?,
        created_at: timestamp(row, 8)?,
        modified_at: timestamp(row, 9)?,
        notes: row.get(10)?,
        notes_html: row.get(11)?,
        recurring: row.get(12)?,
        has_attachments: row.get(13)?,
        steps_loaded: row.get(14)?,
        steps_done: row.get(15)?,
        steps_total: row.get(16)?,
    })
}

fn timestamp(row: &Row, column: usize) -> rusqlite::Result<Timestamp> {
    let millis = row.get(column)?;
    Timestamp::from_millisecond(millis).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(column, Type::Integer, Box::new(error))
    })
}

fn optional_timestamp(row: &Row, column: usize) -> rusqlite::Result<Option<Timestamp>> {
    let millis = row.get::<_, Option<i64>>(column)?;
    millis
        .map(|millis| {
            Timestamp::from_millisecond(millis).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(column, Type::Integer, Box::new(error))
            })
        })
        .transpose()
}

fn escape_like(query: &str) -> String {
    query
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_valid() {
        MIGRATIONS.validate().unwrap();
    }
}
