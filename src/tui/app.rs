//! The UI's state: what is where, what is picked, and what a key asks for.

use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

use jiff::{Timestamp, Zoned, civil::Date, tz::TimeZone};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    core::Command,
    model::{Task, TaskDraft, TaskId, TaskList},
    tui::{
        details::Details,
        dialog::{Confirm, ConfirmAction, Dialog, DueFor, MovePicker, NameFor, NamePrompt},
        keymap::Keymap,
        list_edit::parse_due,
        screen::Cursor,
        search::Search,
        task_form::{Field as FormField, Request as FormRequest, TaskForm},
        ui::{Hit, Mouse},
        vim::Register,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pane {
    #[default]
    Lists,
    Tasks,
    Details,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Smart {
    Important,
    Planned,
    #[default]
    All,
}

impl Smart {
    pub fn name(self) -> &'static str {
        match self {
            Smart::Important => "Important",
            Smart::Planned => "Planned",
            Smart::All => "All",
        }
    }

    pub fn planned(self) -> bool {
        self == Smart::Planned
    }

    /// Reads a name kept in the session file.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Important" => Some(Smart::Important),
            "Planned" => Some(Smart::Planned),
            "All" => Some(Smart::All),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    Smart(Smart),
    List {
        account: usize,
        id: crate::model::ListId,
    },
}

impl Default for View {
    fn default() -> Self {
        View::Smart(Smart::All)
    }
}

impl View {
    pub fn name(&self, accounts: &[AccountView]) -> String {
        match self {
            View::Smart(smart) => smart.name().to_owned(),
            View::List { account, id } => accounts
                .get(*account)
                .and_then(|account| account.lists.iter().find(|list| &list.id == id))
                .map(|list| list.name.clone())
                .unwrap_or_default(),
        }
    }
}

/// What the lists pane knows about one account.
#[derive(Debug, Clone, Default)]
pub struct AccountView {
    pub address: String,
    pub lists: Vec<TaskList>,
}

#[derive(Debug, Clone)]
pub enum ListRow {
    Favorite { account: usize, list: TaskList },
    Smart(Smart),
    Account(String),
    List { account: usize, list: TaskList },
}

impl ListRow {
    pub fn selectable(&self) -> bool {
        !matches!(self, ListRow::Account(_))
    }

    fn is_smart(&self, wanted: Smart) -> bool {
        matches!(self, ListRow::Smart(smart) if *smart == wanted)
    }
}

/// A task as the tasks pane shows it.
#[derive(Debug, Clone)]
pub struct TaskRow {
    pub task: Task,
    pub list_name: String,
    pub account: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Info(String),
    Error(String),
}

#[derive(Debug)]
pub enum Effect {
    None,
    ReloadLists,
    ReloadTasks,
    Send { account: usize, command: Command },
    SendAll(Command),
    Copy(String),
    InstallUpdate,
    Quit,
}

pub struct AppState {
    pub accounts: Vec<AccountView>,
    /// Lists shown again above the smart views, in the order they were added.
    pub favorites: Vec<crate::model::ListId>,
    pub rows: Vec<ListRow>,
    pub selected_list: usize,
    pub view: View,
    pub tasks: Vec<TaskRow>,
    pub selected_task: usize,
    pub details: Details,
    pub focus: Pane,
    pub zoomed: bool,
    pub lists_hidden: bool,
    pub details_below: bool,
    pub completed_hidden: bool,
    pub collapsed_groups: HashSet<String>,
    pub marks: HashSet<TaskId>,
    pub page: usize,
    pub dialog: Option<Dialog>,
    pub form: Option<TaskForm>,
    pub search: Option<Search>,
    pub status: Option<Status>,
    pub status_at: Option<Instant>,
    /// Open counts for Important, Planned and All, filled by the task boxes.
    pub smart_counts: (u32, u32, u32),
    pub clock: Option<Zoned>,
    pub should_quit: bool,
    pub update: Option<semver::Version>,
    pub installed: Option<semver::Version>,
    pub register: Register,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            accounts: Vec::new(),
            favorites: Vec::new(),
            rows: vec![
                ListRow::Smart(Smart::Important),
                ListRow::Smart(Smart::Planned),
                ListRow::Smart(Smart::All),
            ],
            selected_list: 2,
            view: View::Smart(Smart::All),
            tasks: Vec::new(),
            selected_task: 0,
            details: Details::default(),
            focus: Pane::Lists,
            zoomed: false,
            lists_hidden: false,
            details_below: false,
            completed_hidden: false,
            collapsed_groups: HashSet::new(),
            marks: HashSet::new(),
            page: 10,
            dialog: None,
            form: None,
            search: None,
            status: None,
            status_at: None,
            smart_counts: (0, 0, 0),
            clock: None,
            should_quit: false,
            update: None,
            installed: None,
            register: Register::default(),
        }
    }
}

impl AppState {
    /// Today, as the injected clock says, or the system's when there is none.
    pub fn today(&self) -> Date {
        self.clock
            .as_ref()
            .map_or_else(|| Zoned::now().date(), |clock| clock.date())
    }

    pub fn zone(&self) -> TimeZone {
        self.clock.as_ref().map_or_else(
            || Zoned::now().time_zone().clone(),
            |clock| clock.time_zone().clone(),
        )
    }

    pub fn now(&self) -> Timestamp {
        self.clock
            .as_ref()
            .map_or_else(Timestamp::now, |clock| clock.timestamp())
    }

    /// Replaces every account's lists and rebuilds the rows, keeping the
    /// view on what it was when it still exists.
    pub fn set_accounts(&mut self, accounts: Vec<(String, Vec<TaskList>)>) {
        self.accounts = accounts
            .into_iter()
            .map(|(address, lists)| AccountView { address, lists })
            .collect();
        self.rebuild_rows();
    }

    pub(crate) fn rebuild_rows(&mut self) {
        let several = self.accounts.len() > 1;
        let mut rows: Vec<ListRow> = self
            .favorite_rows()
            .into_iter()
            .map(|(account, list)| ListRow::Favorite { account, list })
            .collect();
        rows.extend([
            ListRow::Smart(Smart::Important),
            ListRow::Smart(Smart::Planned),
            ListRow::Smart(Smart::All),
        ]);
        for (index, account) in self.accounts.iter().enumerate() {
            if several {
                rows.push(ListRow::Account(account.address.clone()));
            }
            for list in &account.lists {
                rows.push(ListRow::List {
                    account: index,
                    list: list.clone(),
                });
            }
        }
        self.rows = rows;
        // Keep the view where it was, or fall back to every open task.
        let keep = self
            .rows
            .iter()
            .position(|row| self.row_matches_view(row))
            .or_else(|| self.rows.iter().position(|row| row.is_smart(Smart::All)))
            .unwrap_or(0);
        self.selected_list = keep;
        if !self.rows[keep].selectable() {
            self.view = View::Smart(Smart::All);
        }
    }

    /// The favorites whose lists the accounts still have, with the account
    /// each is in. A favorite whose list is missing is left out but kept.
    pub(crate) fn favorite_rows(&self) -> Vec<(usize, TaskList)> {
        self.favorites
            .iter()
            .filter_map(|id| {
                self.accounts
                    .iter()
                    .enumerate()
                    .find_map(|(account, view)| {
                        view.lists
                            .iter()
                            .find(|list| &list.id == id)
                            .map(|list| (account, list.clone()))
                    })
            })
            .collect()
    }

    fn row_matches_view(&self, row: &ListRow) -> bool {
        match (row, &self.view) {
            (ListRow::Smart(smart), View::Smart(view)) => smart == view,
            (
                ListRow::Favorite { account, list } | ListRow::List { account, list },
                View::List {
                    account: viewed,
                    id,
                },
            ) => account == viewed && &list.id == id,
            _ => false,
        }
    }

    /// The lists pane's selected row, when it is a task list.
    pub fn selected_list(&self) -> Option<(usize, TaskList)> {
        match self.rows.get(self.selected_list) {
            Some(ListRow::Favorite { account, list } | ListRow::List { account, list }) => {
                Some((*account, list.clone()))
            }
            _ => None,
        }
    }

    pub fn selected_smart(&self) -> Option<Smart> {
        match self.rows.get(self.selected_list) {
            Some(ListRow::Smart(smart)) => Some(*smart),
            _ => None,
        }
    }

    /// Picks the row at `index`, asking for its tasks when it is a view.
    pub fn select_list(&mut self, index: usize) -> Effect {
        if !self.rows.get(index).is_some_and(ListRow::selectable) {
            return Effect::None;
        }
        let changed = index != self.selected_list;
        self.selected_list = index;
        self.view = match &self.rows[index] {
            ListRow::Smart(smart) => View::Smart(*smart),
            ListRow::Favorite { account, list } | ListRow::List { account, list } => View::List {
                account: *account,
                id: list.id.clone(),
            },
            ListRow::Account(_) => return Effect::None,
        };
        self.selected_task = 0;
        self.marks.clear();
        if changed {
            Effect::ReloadTasks
        } else {
            Effect::None
        }
    }

    pub fn set_tasks(&mut self, tasks: Vec<TaskRow>) {
        self.tasks = tasks;
        let visible = self.visible_tasks();
        self.selected_task = self.selected_task.min(visible.len().saturating_sub(1));
        self.marks
            .retain(|id| self.tasks.iter().any(|row| &row.task.id == id));
    }

    pub fn selected_task(&self) -> Option<&TaskRow> {
        let visible = self.visible_task_indexes();
        visible
            .get(self.selected_task.min(visible.len().saturating_sub(1)))
            .and_then(|index| self.tasks.get(*index))
    }

    fn visible_task_indexes(&self) -> Vec<usize> {
        let today = self.today();
        let mut seen: Vec<String> = Vec::new();
        let mut visible = Vec::new();
        for (index, row) in self.tasks.iter().enumerate() {
            if self.completed_hidden && !row.task.status.is_open() {
                continue;
            }
            if self.searching() {
                visible.push(index);
                continue;
            }
            let group = crate::tui::groups::group_of(&row.task, &self.view, &row.list_name, today);
            let key = group.key();
            if !seen.contains(&key) {
                seen.push(key.clone());
            }
            if !self.collapsed_groups.contains(&key) {
                visible.push(index);
            }
        }
        visible
    }

    /// The tasks actually shown, in display order.
    pub fn visible_tasks(&self) -> Vec<&TaskRow> {
        self.visible_task_indexes()
            .into_iter()
            .filter_map(|index| self.tasks.get(index))
            .collect()
    }

    /// Every task the tasks pane shows, marked ones first, for acting on.
    pub fn target_tasks(&self) -> Vec<&TaskRow> {
        let marked: Vec<&TaskRow> = self
            .visible_tasks()
            .into_iter()
            .filter(|row| self.marks.contains(&row.task.id))
            .collect();
        if marked.is_empty() {
            self.selected_task().into_iter().collect()
        } else {
            marked
        }
    }

    pub fn targets(&self) -> Vec<(usize, crate::model::ListId, TaskId)> {
        self.target_tasks()
            .into_iter()
            .map(|row| (row.account, row.task.list.clone(), row.task.id.clone()))
            .collect()
    }

    pub fn searching(&self) -> bool {
        self.search.is_some()
    }

    pub fn selected_list_name(&self) -> String {
        self.view.name(&self.accounts)
    }

    pub fn status_info(&mut self, text: impl Into<String>) {
        self.status = Some(Status::Info(text.into()));
        self.status_at = Some(Instant::now());
    }

    pub fn status_error(&mut self, text: impl Into<String>) {
        self.status = Some(Status::Error(text.into()));
        self.status_at = Some(Instant::now());
    }

    pub fn expire_status(&mut self, now: Instant) {
        if self
            .status_at
            .is_some_and(|at| now.duration_since(at) > Duration::from_secs(5))
        {
            self.status = None;
            self.status_at = None;
        }
    }

    pub fn cursor(&self) -> Cursor {
        if let Some(form) = &self.form {
            return form.cursor();
        }
        if self.search.as_ref().is_some_and(|search| search.typing()) {
            return Cursor::Bar;
        }
        if let Some(Dialog::Name(prompt)) = &self.dialog
            && prompt.takes_text()
        {
            return Cursor::Bar;
        }
        if let Some(Dialog::Due(_)) = &self.dialog {
            return Cursor::Bar;
        }
        if self.focus == Pane::Details && self.dialog.is_none() && self.form.is_none() {
            return match self.details.viewer.mode() {
                crate::tui::vim::VimMode::Insert => Cursor::Bar,
                _ => Cursor::Block,
            };
        }
        Cursor::Default
    }

    /// Hands a key to whatever is taking keys: a dialog, the search line, the
    /// task form, or the panes.
    pub fn on_key(&mut self, key: KeyEvent, keymap: &mut Keymap) -> Effect {
        if key.kind != ratatui::crossterm::event::KeyEventKind::Press {
            return Effect::None;
        }
        if let Some(dialog) = self.dialog.take() {
            return self.dialog_key(dialog, key);
        }
        if let Some(form) = self.form.take() {
            return self.form_key(form, key);
        }
        if self.search.as_ref().is_some_and(Search::typing) {
            return self.search_key(key);
        }
        if self.focus == Pane::Details {
            match self.details.viewer.key(key, self.page, &mut self.register) {
                crate::tui::viewer::Request::Copy(text) => return Effect::Copy(text),
                crate::tui::viewer::Request::None => return Effect::None,
                crate::tui::viewer::Request::Unhandled => {}
            }
            let effect = self.details_key(key);
            if !matches!(effect, Effect::None) {
                return effect;
            }
        }
        let Some(action) = keymap.action_for(key) else {
            return if self.focus == Pane::Tasks && key.code == KeyCode::Esc {
                self.clear_search()
            } else {
                Effect::None
            };
        };
        self.update(action)
    }

    /// A click or the wheel over the panes.
    pub fn on_mouse(&mut self, mouse: Mouse, hit: Option<Hit>) -> Effect {
        let Some(hit) = hit else {
            return Effect::None;
        };
        match mouse {
            Mouse::Click => {
                self.focus = hit.pane;
                if let Some(row) = hit.row {
                    match hit.pane {
                        Pane::Lists => self.select_list(row),
                        Pane::Tasks => {
                            self.selected_task = row;
                            Effect::None
                        }
                        Pane::Details => Effect::None,
                    }
                } else {
                    Effect::None
                }
            }
            Mouse::ScrollUp => {
                self.focus = hit.pane;
                self.step_selection(-1);
                self.after_selection()
            }
            Mouse::ScrollDown => {
                self.focus = hit.pane;
                self.step_selection(1);
                self.after_selection()
            }
        }
    }

    fn details_key(&mut self, key: KeyEvent) -> Effect {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return Effect::None;
        }
        match key.code {
            KeyCode::Right => {
                self.select_next();
                Effect::ReloadTasks
            }
            KeyCode::Left => {
                self.select_prev();
                Effect::ReloadTasks
            }
            KeyCode::Char('x') => self.toggle_step_or_task(),
            KeyCode::Char('s') => self.add_step(),
            KeyCode::F(2) => self.rename_step_or_task(),
            KeyCode::Char('d') => self.delete_step_or_task(),
            KeyCode::Char('i') | KeyCode::Char('e') => self.edit_task(),
            KeyCode::Char('*') => self.toggle_star(),
            KeyCode::Char('t') => self.due_today(),
            KeyCode::Char('T') => self.ask_due(),
            KeyCode::Char('m') => self.move_tasks(),
            _ => Effect::None,
        }
    }

    pub fn update(&mut self, action: crate::model::Action) -> Effect {
        use crate::model::Action::*;
        match action {
            FocusNextPane => self.focus_next(),
            FocusPrevPane => self.focus_prev(),
            SelectNext => {
                self.select_next();
                self.after_selection()
            }
            SelectPrev => {
                self.select_prev();
                self.after_selection()
            }
            PageDown => {
                self.step_selection(self.page as isize);
                self.after_selection()
            }
            PageUp => {
                self.step_selection(-(self.page as isize));
                self.after_selection()
            }
            HalfPageDown => {
                self.step_selection((self.page / 2) as isize);
                self.after_selection()
            }
            HalfPageUp => {
                self.step_selection(-((self.page / 2) as isize));
                self.after_selection()
            }
            Top => {
                self.jump(true);
                self.after_selection()
            }
            Bottom => {
                self.jump(false);
                self.after_selection()
            }
            Open => self.open(),
            Expand => self.expand(),
            Collapse => self.collapse(),
            Quit => {
                self.should_quit = true;
                Effect::Quit
            }
            ToggleZoom => {
                self.zoomed = !self.zoomed;
                Effect::None
            }
            ToggleLists => {
                self.lists_hidden = !self.lists_hidden;
                if self.lists_hidden && self.focus == Pane::Lists {
                    self.focus = Pane::Tasks;
                }
                Effect::None
            }
            ToggleDetailsPlace => {
                self.details_below = !self.details_below;
                Effect::None
            }
            ToggleMark => {
                if self.focus == Pane::Tasks {
                    self.toggle_mark();
                }
                Effect::None
            }
            MarkAll => {
                if self.focus == Pane::Tasks {
                    let ids: Vec<TaskId> = self
                        .visible_tasks()
                        .iter()
                        .map(|row| row.task.id.clone())
                        .collect();
                    for id in ids {
                        self.marks.insert(id);
                    }
                }
                Effect::None
            }
            ClearMarks => {
                self.marks.clear();
                if self.search.is_some() && self.focus == Pane::Tasks {
                    return self.clear_search();
                }
                Effect::None
            }
            Refresh => {
                self.status_info("Syncing…");
                Effect::SendAll(Command::SyncAll)
            }
            Help => {
                self.dialog = Some(Dialog::Help);
                Effect::None
            }
            Search => {
                self.search = Some(crate::tui::search::Search::default());
                Effect::None
            }
            Update => match self.update.clone() {
                Some(version) => {
                    self.dialog = Some(Dialog::Confirm(crate::tui::dialog::Confirm {
                        prompt: format!(
                            "Update TUIList to {version}? It runs from the next start."
                        ),
                        action: ConfirmAction::Update,
                    }));
                    Effect::None
                }
                None => {
                    self.status_info("TUIList is up to date.");
                    Effect::None
                }
            },
            ManageAccounts => {
                self.dialog = Some(Dialog::Accounts(crate::tui::account_manager::Accounts::of(
                    &self.accounts,
                )));
                Effect::None
            }
            Add => self.add(),
            Rename => self.rename(),
            Delete => self.delete(),
            ToggleComplete => self.toggle_complete(),
            ToggleStar if self.focus == Pane::Lists => self.toggle_favorite(),
            ToggleStar => self.toggle_star(),
            Edit => self.edit_task(),
            DueToday => self.due_today(),
            AskDue => self.ask_due(),
            AddStep => self.add_step(),
            Move => self.move_tasks(),
            Copy => self
                .selected_task()
                .map_or(Effect::None, |row| Effect::Copy(row.task.title.clone())),
            ToggleCompletedShown => {
                if self.focus == Pane::Tasks {
                    self.completed_hidden = !self.completed_hidden;
                }
                Effect::None
            }
        }
    }

    fn after_selection(&mut self) -> Effect {
        Effect::ReloadTasks
    }

    fn focus_next(&mut self) -> Effect {
        self.focus = match self.focus {
            Pane::Lists if self.lists_hidden => Pane::Tasks,
            Pane::Lists => Pane::Tasks,
            Pane::Tasks => Pane::Details,
            Pane::Details => Pane::Lists,
        };
        Effect::None
    }

    fn focus_prev(&mut self) -> Effect {
        self.focus = match self.focus {
            Pane::Lists => Pane::Details,
            Pane::Tasks if self.lists_hidden => Pane::Details,
            Pane::Tasks => Pane::Lists,
            Pane::Details => Pane::Tasks,
        };
        Effect::None
    }

    fn select_next(&mut self) -> Effect {
        self.step_selection(1)
    }

    fn select_prev(&mut self) -> Effect {
        self.step_selection(-1)
    }

    fn step_selection(&mut self, delta: isize) -> Effect {
        match self.focus {
            Pane::Lists => {
                if delta == 0 {
                    return Effect::None;
                }
                let mut index = self.selected_list;
                loop {
                    let next = index.saturating_add_signed(delta);
                    if next >= self.rows.len() {
                        if delta > 0 {
                            break;
                        }
                        index = self.rows.len();
                    }
                    if delta < 0 && index == 0 {
                        break;
                    }
                    index = next;
                    if index >= self.rows.len() {
                        break;
                    }
                    if self.rows[index].selectable() {
                        return self.select_list(index);
                    }
                }
                Effect::None
            }
            Pane::Tasks | Pane::Details => {
                let len = self.visible_tasks().len();
                self.selected_task = self
                    .selected_task
                    .saturating_add_signed(delta)
                    .min(len.saturating_sub(1));
                Effect::None
            }
        }
    }

    fn jump(&mut self, top: bool) -> Effect {
        match self.focus {
            Pane::Lists => {
                let index = if top {
                    (0..self.rows.len()).find(|index| self.rows[*index].selectable())
                } else {
                    (0..self.rows.len())
                        .rev()
                        .find(|index| self.rows[*index].selectable())
                };
                index.map_or(Effect::None, |index| self.select_list(index))
            }
            Pane::Tasks | Pane::Details => {
                self.selected_task = if top {
                    0
                } else {
                    self.visible_tasks().len().saturating_sub(1)
                };
                Effect::None
            }
        }
    }

    fn open(&mut self) -> Effect {
        match self.focus {
            Pane::Lists => {
                self.focus = Pane::Tasks;
                self.selected_task = 0;
                self.after_selection()
            }
            Pane::Tasks => {
                self.focus = Pane::Details;
                Effect::None
            }
            Pane::Details => Effect::None,
        }
    }

    fn expand(&mut self) -> Effect {
        if self.focus == Pane::Lists {
            self.focus = Pane::Tasks;
        } else if self.focus == Pane::Tasks {
            self.set_group_folded(false);
        } else if self.focus == Pane::Details {
            self.select_next();
            return Effect::ReloadTasks;
        }
        Effect::None
    }

    fn collapse(&mut self) -> Effect {
        if self.focus == Pane::Tasks {
            self.set_group_folded(true);
        } else if self.focus == Pane::Details {
            self.select_prev();
            return Effect::ReloadTasks;
        }
        Effect::None
    }

    fn set_group_folded(&mut self, folded: bool) -> bool {
        let Some(row) = self.selected_task() else {
            return false;
        };
        let group =
            crate::tui::groups::group_of(&row.task, &self.view, &row.list_name, self.today());
        if matches!(group, crate::tui::groups::Group::Plain) {
            return false;
        }
        let key = group.key();
        if folded {
            self.collapsed_groups.insert(key);
        } else {
            self.collapsed_groups.remove(&key);
        }
        true
    }

    fn toggle_mark(&mut self) {
        if let Some(row) = self.selected_task() {
            let id = row.task.id.clone();
            if !self.marks.remove(&id) {
                self.marks.insert(id);
            }
        }
        self.select_next();
    }

    fn add(&mut self) -> Effect {
        match self.focus {
            Pane::Lists => {
                self.dialog = Some(Dialog::Name(NamePrompt::new(
                    "New list",
                    " Name:  ",
                    "",
                    NameFor::NewList,
                )));
                Effect::None
            }
            Pane::Tasks => {
                let Some((account, list)) = self.add_target() else {
                    self.status_error("No list to add a task to");
                    return Effect::None;
                };
                self.dialog = Some(Dialog::Name(NamePrompt::new(
                    "New task",
                    " Title:  ",
                    "",
                    NameFor::NewTask {
                        account,
                        list: list.id,
                    },
                )));
                Effect::None
            }
            Pane::Details => self.add_step(),
        }
    }

    /// The default list a task added in a smart view goes to.
    fn add_target(&self) -> Option<(usize, TaskList)> {
        if let Some((account, list)) = self.selected_list() {
            return Some((account, list));
        }
        let account = self.accounts.first()?;
        let list = account
            .lists
            .iter()
            .find(|list| list.well_known == Some(crate::model::WellKnown::DefaultList))
            .or_else(|| account.lists.first())?;
        let index = self
            .accounts
            .iter()
            .position(|candidate| candidate.address == account.address)?;
        Some((index, list.clone()))
    }

    fn rename(&mut self) -> Effect {
        match self.focus {
            Pane::Lists => {
                let Some((account, list)) = self.selected_list() else {
                    self.status_info("Smart lists cannot be renamed");
                    return Effect::None;
                };
                if list.well_known.is_some() {
                    self.status_info("This list cannot be renamed");
                    return Effect::None;
                }
                self.dialog = Some(Dialog::Name(NamePrompt::new(
                    "Rename",
                    " Name:  ",
                    &list.name,
                    NameFor::RenameList {
                        account,
                        list: list.id,
                    },
                )));
                Effect::None
            }
            Pane::Tasks => {
                let Some(row) = self.selected_task().cloned() else {
                    return Effect::None;
                };
                self.dialog = Some(Dialog::Name(NamePrompt::new(
                    "Rename",
                    " Title:  ",
                    &row.task.title,
                    NameFor::RenameTask {
                        account: row.account,
                        list: row.task.list,
                        task: row.task.id,
                    },
                )));
                Effect::None
            }
            Pane::Details => self.rename_step_or_task(),
        }
    }

    fn rename_step_or_task(&mut self) -> Effect {
        if let Some((step, _)) = self.step_under_cursor() {
            let Some(row) = self.selected_task().cloned() else {
                return Effect::None;
            };
            let name = self.details.step_name(&step).unwrap_or_default();
            self.dialog = Some(Dialog::Name(NamePrompt::new(
                "Rename",
                " Step:  ",
                &name,
                NameFor::RenameStep {
                    account: row.account,
                    list: row.task.list,
                    task: row.task.id,
                    step,
                },
            )));
            return Effect::None;
        }
        self.rename_from_details()
    }

    fn rename_from_details(&mut self) -> Effect {
        let Some(row) = self.selected_task().cloned() else {
            return Effect::None;
        };
        self.dialog = Some(Dialog::Name(NamePrompt::new(
            "Rename",
            " Title:  ",
            &row.task.title,
            NameFor::RenameTask {
                account: row.account,
                list: row.task.list,
                task: row.task.id,
            },
        )));
        Effect::None
    }

    fn delete(&mut self) -> Effect {
        match self.focus {
            Pane::Lists => {
                let Some((_, list)) = self.selected_list() else {
                    self.status_info("Smart lists cannot be deleted");
                    return Effect::None;
                };
                if list.well_known.is_some() {
                    self.status_info("This list cannot be deleted");
                    return Effect::None;
                }
                let tasks = self
                    .accounts
                    .iter()
                    .enumerate()
                    .find(|(_, account)| {
                        account
                            .lists
                            .iter()
                            .any(|candidate| candidate.id == list.id)
                    })
                    .map(|(index, _)| index)
                    .and_then(|account| self.selected_list().map(|(_, list)| (account, list)));
                let Some((account, list)) = tasks else {
                    return Effect::None;
                };
                let count = list.open;
                let prompt = if count == 0 {
                    format!("Delete {}? This cannot be undone.", list.name)
                } else {
                    format!(
                        "Delete {} and its {} task{}? This cannot be undone.",
                        list.name,
                        count,
                        if count == 1 { "" } else { "s" }
                    )
                };
                self.dialog = Some(Dialog::Confirm(Confirm {
                    prompt,
                    action: ConfirmAction::DeleteList {
                        account,
                        list: list.id,
                    },
                }));
                Effect::None
            }
            Pane::Tasks => self.confirm_delete_tasks(),
            Pane::Details => self.delete_step_or_task(),
        }
    }

    fn confirm_delete_tasks(&mut self) -> Effect {
        let targets = self.targets();
        if targets.is_empty() {
            return Effect::None;
        }
        let count = targets.len();
        let prompt = format!(
            "Delete {count} task{}? This cannot be undone.",
            if count == 1 { "" } else { "s" }
        );
        self.dialog = Some(Dialog::Confirm(Confirm {
            prompt,
            action: ConfirmAction::DeleteTasks(targets),
        }));
        Effect::None
    }

    fn delete_step_or_task(&mut self) -> Effect {
        if let Some((step, _)) = self.step_under_cursor() {
            let Some(row) = self.selected_task().cloned() else {
                return Effect::None;
            };
            self.dialog = Some(Dialog::Confirm(Confirm {
                prompt: "Delete 1 step? This cannot be undone.".to_owned(),
                action: ConfirmAction::DeleteStep {
                    account: row.account,
                    list: row.task.list,
                    task: row.task.id,
                    step,
                },
            }));
            return Effect::None;
        }
        self.confirm_delete_tasks()
    }

    fn toggle_complete(&mut self) -> Effect {
        if self.focus == Pane::Details
            && let Some((step, checked)) = self.step_under_cursor()
        {
            let Some(row) = self.selected_task().cloned() else {
                return Effect::None;
            };
            return Effect::Send {
                account: row.account,
                command: Command::UpdateStep {
                    list: row.task.list,
                    task: row.task.id,
                    step,
                    name: None,
                    checked: Some(!checked),
                },
            };
        }
        if self.focus != Pane::Tasks && self.focus != Pane::Details {
            return Effect::None;
        }
        let targets = self.targets();
        if targets.is_empty() {
            return Effect::None;
        }
        let complete = !self
            .target_tasks()
            .iter()
            .all(|row| !row.task.status.is_open());
        let mut effect = Effect::None;
        for (account, list, task) in targets {
            effect = Effect::Send {
                account,
                command: Command::UpdateTasks {
                    tasks: vec![(list, task)],
                    draft: TaskDraft {
                        completed: Some(complete),
                        ..TaskDraft::default()
                    },
                },
            };
        }
        self.marks.clear();
        effect
    }

    fn toast_sent(&mut self, what: &str) {
        self.status_info(what);
    }

    fn toggle_step_or_task(&mut self) -> Effect {
        self.toggle_complete()
    }

    fn toggle_star(&mut self) -> Effect {
        if self.focus != Pane::Tasks && self.focus != Pane::Details {
            return Effect::None;
        }
        let targets = self.targets();
        if targets.is_empty() {
            return Effect::None;
        }
        let star = !self
            .target_tasks()
            .iter()
            .all(|row| row.task.importance.is_starred());
        let mut effect = Effect::None;
        for (account, list, task) in targets {
            effect = Effect::Send {
                account,
                command: Command::UpdateTasks {
                    tasks: vec![(list, task)],
                    draft: TaskDraft {
                        starred: Some(star),
                        ..TaskDraft::default()
                    },
                },
            };
        }
        self.marks.clear();
        effect
    }

    fn due_today(&mut self) -> Effect {
        if self.focus != Pane::Tasks && self.focus != Pane::Details {
            return Effect::None;
        }
        let targets = self.targets();
        if targets.is_empty() {
            return Effect::None;
        }
        let today = self.today();
        let clear = self
            .target_tasks()
            .iter()
            .all(|row| row.task.due == Some(today));
        let due = if clear { None } else { Some(today) };
        let mut effect = Effect::None;
        for (account, list, task) in targets {
            effect = Effect::Send {
                account,
                command: Command::UpdateTasks {
                    tasks: vec![(list, task)],
                    draft: TaskDraft {
                        due: Some(due),
                        ..TaskDraft::default()
                    },
                },
            };
        }
        self.marks.clear();
        effect
    }

    fn ask_due(&mut self) -> Effect {
        if self.focus != Pane::Tasks && self.focus != Pane::Details {
            return Effect::None;
        }
        let targets = self.targets();
        if targets.is_empty() {
            return Effect::None;
        }
        let account = targets[0].0;
        self.dialog = Some(Dialog::Due(crate::tui::dialog::DuePrompt::new(
            DueFor::Tasks {
                account,
                tasks: targets
                    .into_iter()
                    .filter(|(owner, _, _)| *owner == account)
                    .map(|(_, list, task)| (list, task))
                    .collect(),
            },
        )));
        Effect::None
    }

    fn edit_task(&mut self) -> Effect {
        let Some(row) = self.selected_task().cloned() else {
            return Effect::None;
        };
        self.form = Some(TaskForm::open(row.task, self.zone(), self.today()));
        Effect::None
    }

    fn add_step(&mut self) -> Effect {
        let Some(row) = self.selected_task().cloned() else {
            return Effect::None;
        };
        self.dialog = Some(Dialog::Name(NamePrompt::new(
            "New step",
            " Step:  ",
            "",
            NameFor::NewStep {
                account: row.account,
                list: row.task.list,
                task: row.task.id,
            },
        )));
        Effect::None
    }

    fn move_tasks(&mut self) -> Effect {
        let targets = self.targets();
        if targets.is_empty() {
            return Effect::None;
        }
        let account = targets[0].0;
        if targets.iter().any(|(owner, _, _)| *owner != account) {
            self.status_error("Tasks can only be moved within their own account");
            return Effect::None;
        }
        let Some(account_view) = self.accounts.get(account) else {
            return Effect::None;
        };
        let choices = account_view
            .lists
            .iter()
            .map(|list| (list.id.clone(), list.name.clone()))
            .collect();
        self.dialog = Some(Dialog::Move(MovePicker::new(
            account,
            targets
                .into_iter()
                .map(|(_, list, task)| (list, task))
                .collect(),
            choices,
        )));
        Effect::None
    }

    fn clear_search(&mut self) -> Effect {
        if self.search.take().is_some() {
            self.selected_task = 0;
            self.marks.clear();
            Effect::ReloadTasks
        } else {
            Effect::None
        }
    }

    /// The step the details cursor is on, and whether it is checked.
    fn step_under_cursor(&self) -> Option<(crate::model::StepId, bool)> {
        if self.focus != Pane::Details {
            return None;
        }
        let row = self.details.viewer.cursor().0;
        self.details.step_at(row)
    }

    fn search_key(&mut self, key: KeyEvent) -> Effect {
        let Some(search) = &mut self.search else {
            return Effect::None;
        };
        match key.code {
            KeyCode::Enter => {
                search.finish();
                self.focus = Pane::Tasks;
                self.selected_task = 0;
                Effect::None
            }
            KeyCode::Esc => self.clear_search(),
            _ => {
                if search.input.key(key) {
                    self.selected_task = 0;
                    Effect::ReloadTasks
                } else {
                    Effect::None
                }
            }
        }
    }

    fn form_key(&mut self, mut form: TaskForm, key: KeyEvent) -> Effect {
        let request = form.key(key, self.page, &mut self.register);
        match request {
            FormRequest::None => {
                self.form = Some(form);
                Effect::None
            }
            FormRequest::Copy(text) => {
                self.form = Some(form);
                Effect::Copy(text)
            }
            FormRequest::Save(draft) => {
                let Some(row) = self.selected_task().cloned() else {
                    return Effect::None;
                };
                self.form = None;
                Effect::Send {
                    account: row.account,
                    command: Command::UpdateTasks {
                        tasks: vec![(row.task.list, row.task.id)],
                        draft,
                    },
                }
            }
            FormRequest::Cancel => {
                self.form = None;
                Effect::None
            }
            FormRequest::AskDiscard => {
                self.form = Some(form);
                self.dialog = Some(Dialog::Confirm(Confirm {
                    prompt: "Close the task and lose your changes?".to_owned(),
                    action: ConfirmAction::DiscardForm,
                }));
                Effect::None
            }
        }
    }

    fn dialog_key(&mut self, mut dialog: Dialog, key: KeyEvent) -> Effect {
        match &mut dialog {
            Dialog::Help => Effect::None,
            Dialog::Accounts(accounts) => {
                if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
                    return Effect::None;
                }
                accounts.key(key);
                self.dialog = Some(dialog);
                Effect::None
            }
            Dialog::Confirm(confirm) => {
                let accepted = matches!(key.code, KeyCode::Char('y') | KeyCode::Enter);
                if !accepted {
                    return Effect::None;
                }
                let action = confirm.action.clone();
                self.confirm(action)
            }
            Dialog::Name(prompt) => match key.code {
                KeyCode::Esc => Effect::None,
                KeyCode::Enter => {
                    let text = prompt.input.text().trim().to_owned();
                    if text.is_empty() {
                        self.dialog = Some(dialog);
                        return Effect::None;
                    }
                    let purpose = prompt.purpose.clone();
                    self.save_name(purpose, text)
                }
                _ => {
                    prompt.input.key(key);
                    self.dialog = Some(dialog);
                    Effect::None
                }
            },
            Dialog::Due(due) => match key.code {
                KeyCode::Esc => Effect::None,
                KeyCode::Enter => {
                    let text = due.input.text().trim().to_owned();
                    match parse_due(&text, self.today()) {
                        Ok(value) => {
                            let purpose = due.purpose.clone();
                            self.save_due(purpose, value)
                        }
                        Err(error) => {
                            due.error = Some(error);
                            self.dialog = Some(dialog);
                            Effect::None
                        }
                    }
                }
                _ => {
                    due.input.key(key);
                    due.error = None;
                    self.dialog = Some(dialog);
                    Effect::None
                }
            },
            Dialog::Move(picker) => match picker.key(key) {
                crate::tui::dialog::MoveRequest::None => {
                    self.dialog = Some(dialog);
                    Effect::None
                }
                crate::tui::dialog::MoveRequest::Cancel => Effect::None,
                crate::tui::dialog::MoveRequest::Move(to) => {
                    let tasks = picker.tasks.clone();
                    let account = picker.account;
                    Effect::Send {
                        account,
                        command: Command::MoveTasks { tasks, to },
                    }
                }
            },
        }
    }

    fn confirm(&mut self, action: ConfirmAction) -> Effect {
        match action {
            ConfirmAction::DeleteList { account, list } => Effect::Send {
                account,
                command: Command::DeleteList(list),
            },
            ConfirmAction::DeleteTasks(tasks) => {
                self.marks.clear();
                let mut by_account: Vec<(usize, Vec<(crate::model::ListId, TaskId)>)> = Vec::new();
                for (account, list, task) in tasks {
                    match by_account.iter_mut().find(|(owner, _)| *owner == account) {
                        Some((_, group)) => group.push((list, task)),
                        None => by_account.push((account, vec![(list, task)])),
                    }
                }
                let mut effect = Effect::None;
                for (account, tasks) in by_account {
                    effect = Effect::Send {
                        account,
                        command: Command::DeleteTasks(tasks),
                    };
                }
                effect
            }
            ConfirmAction::DeleteStep {
                account,
                list,
                task,
                step,
            } => Effect::Send {
                account,
                command: Command::DeleteStep { list, task, step },
            },
            ConfirmAction::DiscardForm => {
                self.form = None;
                Effect::None
            }
            ConfirmAction::Update => Effect::InstallUpdate,
        }
    }

    fn save_name(&mut self, purpose: NameFor, text: String) -> Effect {
        match purpose {
            NameFor::NewList => Effect::Send {
                account: 0,
                command: Command::CreateList(text),
            },
            NameFor::RenameList { account, list } => Effect::Send {
                account,
                command: Command::RenameList { list, name: text },
            },
            NameFor::NewTask { account, list } => {
                self.focus = Pane::Tasks;
                Effect::Send {
                    account,
                    command: Command::CreateTask {
                        list,
                        draft: TaskDraft {
                            title: Some(text),
                            ..TaskDraft::default()
                        },
                    },
                }
            }
            NameFor::RenameTask {
                account,
                list,
                task,
            } => Effect::Send {
                account,
                command: Command::UpdateTasks {
                    tasks: vec![(list, task)],
                    draft: TaskDraft {
                        title: Some(text),
                        ..TaskDraft::default()
                    },
                },
            },
            NameFor::NewStep {
                account,
                list,
                task,
            } => Effect::Send {
                account,
                command: Command::CreateStep {
                    list,
                    task,
                    name: text,
                },
            },
            NameFor::RenameStep {
                account,
                list,
                task,
                step,
            } => Effect::Send {
                account,
                command: Command::UpdateStep {
                    list,
                    task,
                    step,
                    name: Some(text),
                    checked: None,
                },
            },
        }
    }

    fn save_due(&mut self, purpose: DueFor, value: Option<Date>) -> Effect {
        let draft = TaskDraft {
            due: Some(value),
            ..TaskDraft::default()
        };
        match purpose {
            DueFor::Tasks { account, tasks } => Effect::Send {
                account,
                command: Command::UpdateTasks { tasks, draft },
            },
        }
    }

    pub fn set_steps(&mut self, task: &TaskId, steps: &[crate::model::Step]) {
        if self.selected_task().is_some_and(|row| &row.task.id == task) {
            let list_name = self.selected_list_name();
            if let Some(row) = self.selected_task().cloned() {
                self.details
                    .set(&row.task, Some(steps), &list_name, self.zone());
            }
        }
    }

    /// Rebuilds the details pane for the selected task.
    pub fn show_details(&mut self, steps: Option<&[crate::model::Step]>) {
        let list_name = if self.searching() {
            self.selected_task()
                .map_or_else(String::new, |row| row.list_name.clone())
        } else if matches!(self.view, View::Smart(_)) {
            self.selected_list_name()
        } else {
            String::new()
        };
        match self.selected_task().cloned() {
            Some(row) => self.details.set(&row.task, steps, &list_name, self.zone()),
            None => self.details.clear(),
        }
    }

    pub fn hidden_completed(&self) -> bool {
        self.completed_hidden
    }

    pub fn mark_list_summary(&self) -> Option<String> {
        (!self.marks.is_empty()).then(|| format!("  ({} marked) ", self.marks.len()))
    }

    pub fn step_counts(&self) -> (u32, u32) {
        self.selected_task()
            .map_or((0, 0), |row| (row.task.steps_done, row.task.steps_total))
    }

    /// Asks the engine for the selected task's steps when they were never
    /// fetched.
    pub fn load_selected_steps(&self) -> Option<Effect> {
        let row = self.selected_task()?;
        if row.task.steps_loaded {
            return None;
        }
        Some(Effect::Send {
            account: row.account,
            command: Command::LoadSteps {
                list: row.task.list.clone(),
                task: row.task.id.clone(),
            },
        })
    }

    pub fn form_field(&self) -> Option<FormField> {
        self.form.as_ref().map(TaskForm::field)
    }

    /// Called when the dialog now open needs the form gone, and the key was
    /// handled by the dialog.
    pub fn discard_form_if_confirmed(&mut self, accepted: bool) {
        if accepted && self.form.is_some() {
            self.form = None;
        }
    }

    pub fn toast(&mut self, what: &str) {
        self.toast_sent(what);
    }

    pub fn dismiss(&mut self) {
        self.dialog = None;
    }
}
