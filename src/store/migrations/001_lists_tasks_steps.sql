CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);

CREATE TABLE lists (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    well_known TEXT,
    is_owner INTEGER NOT NULL,
    is_shared INTEGER NOT NULL,
    delta_link TEXT);

CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    list_id TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    importance TEXT NOT NULL,
    due TEXT,
    reminder_at INTEGER,
    completed_at INTEGER,
    created_at INTEGER NOT NULL,
    modified_at INTEGER NOT NULL,
    notes TEXT NOT NULL,
    notes_html INTEGER NOT NULL,
    recurring INTEGER NOT NULL,
    has_attachments INTEGER NOT NULL,
    steps_loaded INTEGER NOT NULL DEFAULT 0);

CREATE INDEX tasks_by_list ON tasks(list_id, created_at DESC);

CREATE TABLE steps (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    checked INTEGER NOT NULL,
    created_at INTEGER NOT NULL);

CREATE INDEX steps_by_task ON steps(task_id, created_at);
