//! Drawing the panes, the dialogs, the task form and the status line.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Paragraph, Wrap},
};

use crate::tui::{
    app::{AppState, ListRow, Pane, Smart, TaskRow},
    dialog::{Dialog, MovePicker},
    groups::{self, Group},
    help, numbers,
    task_form::{Field, TaskForm},
    task_list::due_text,
};

/// Wide enough for account addresses and ordinary list names.
const LISTS_WIDTH: u16 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mouse {
    Click,
    ScrollUp,
    ScrollDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    pub pane: Pane,
    pub row: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
pub struct Panes {
    pub lists: Rect,
    pub tasks: Rect,
    pub details: Rect,
}

pub fn draw(frame: &mut Frame, app: &mut AppState) {
    let area = frame.area();
    if area.width < 40 || area.height < 8 {
        frame.render_widget(Paragraph::new("Terminal too small").centered(), area);
        return;
    }
    let [body, status] = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(area);
    let panes = panes(body, app);
    if app.zoomed {
        match app.focus {
            Pane::Lists => draw_lists(frame, panes.lists, app),
            Pane::Tasks => draw_tasks(frame, panes.tasks, app),
            Pane::Details => draw_details(frame, panes.details, app),
        }
    } else {
        draw_lists(frame, panes.lists, app);
        draw_tasks(frame, panes.tasks, app);
        draw_details(frame, panes.details, app);
    }
    if let Some(form) = &app.form {
        draw_form(frame, panes.tasks.union(panes.details), form);
    }
    draw_status(frame, status, app);
    if let Some(dialog) = &app.dialog {
        draw_dialog(frame, area, dialog, app);
    }
}

pub fn panes(area: Rect, app: &AppState) -> Panes {
    if app.zoomed {
        let focused = area;
        let hidden = Rect::new(area.x, area.y, 0, area.height);
        return match app.focus {
            Pane::Lists => Panes {
                lists: focused,
                tasks: hidden,
                details: hidden,
            },
            Pane::Tasks => Panes {
                lists: hidden,
                tasks: focused,
                details: hidden,
            },
            Pane::Details => Panes {
                lists: hidden,
                tasks: hidden,
                details: focused,
            },
        };
    }
    let lists_width = if area.width < 60 { 16 } else { LISTS_WIDTH };
    if app.details_below {
        let [left, right] = Layout::horizontal([
            Constraint::Length(if app.lists_hidden { 0 } else { lists_width }),
            Constraint::Min(1),
        ])
        .areas(area);
        let lists = if app.lists_hidden {
            Rect::new(area.x, area.y, 0, area.height)
        } else {
            left
        };
        let [tasks, details] =
            Layout::vertical([Constraint::Min(1), Constraint::Percentage(40)]).areas(right);
        return Panes {
            lists,
            tasks,
            details,
        };
    }
    if app.lists_hidden {
        let [tasks, details] =
            Layout::horizontal([Constraint::Percentage(55), Constraint::Min(1)]).areas(area);
        Panes {
            lists: Rect::new(area.x, area.y, 0, area.height),
            tasks,
            details,
        }
    } else {
        let [lists, tasks, details] = Layout::horizontal([
            Constraint::Length(lists_width),
            Constraint::Percentage(45),
            Constraint::Min(1),
        ])
        .areas(area);
        Panes {
            lists,
            tasks,
            details,
        }
    }
}

pub fn page_height(area: Rect, app: &AppState) -> usize {
    let panes = panes(area, app);
    let rect = match app.focus {
        Pane::Lists => panes.lists,
        Pane::Tasks => panes.tasks,
        Pane::Details => panes.details,
    };
    usize::from(rect.height.saturating_sub(2)).max(1)
}

pub fn hit(area: Rect, app: &AppState, column: u16, row: u16) -> Option<Hit> {
    let panes = panes(area, app);
    let inside = |rect: Rect| {
        column >= rect.x
            && column < rect.x.saturating_add(rect.width)
            && row >= rect.y
            && row < rect.y.saturating_add(rect.height)
    };
    if inside(panes.lists) {
        let (smart, lists) = list_rects(panes.lists);
        let row = if inside(smart) {
            row.saturating_sub(smart.y + 1) as usize
        } else if inside(lists) {
            3 + row.saturating_sub(lists.y + 1) as usize
        } else {
            return None;
        };
        return Some(Hit {
            pane: Pane::Lists,
            row: Some(row),
        });
    }
    if inside(panes.tasks) {
        return Some(Hit {
            pane: Pane::Tasks,
            row: Some(row.saturating_sub(panes.tasks.y + 1) as usize),
        });
    }
    if inside(panes.details) {
        return Some(Hit {
            pane: Pane::Details,
            row: None,
        });
    }
    None
}

fn list_rects(area: Rect) -> (Rect, Rect) {
    let [smart, lists] = Layout::vertical([Constraint::Length(5), Constraint::Min(0)]).areas(area);
    (smart, lists)
}

fn draw_lists(frame: &mut Frame, area: Rect, app: &AppState) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let (smart_area, lists_area) = list_rects(area);
    let focus = app.focus == Pane::Lists;
    let border = if focus {
        Style::new().fg(Color::Cyan)
    } else {
        Style::new()
    };
    let smart = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(border)
        .title(" Smart ");
    let inner = smart.inner(smart_area);
    frame.render_widget(smart, smart_area);
    let counts = [app.smart_counts.0, app.smart_counts.1, app.smart_counts.2];
    for (index, smart) in [Smart::Important, Smart::Planned, Smart::All]
        .into_iter()
        .enumerate()
    {
        if index >= inner.height as usize {
            break;
        }
        let selected = app.selected_list == index && focus;
        let count = counts[index];
        let mut text = format!("  {}", smart.name());
        if count > 0 {
            text.push_str(&format!(" ({count})"));
        }
        let line = Line::from(text);
        let line = if selected {
            line.style(Style::new().add_modifier(Modifier::REVERSED))
        } else {
            line
        };
        frame.render_widget(
            Paragraph::new(line),
            Rect::new(inner.x, inner.y + index as u16, inner.width, 1),
        );
    }

    let lists = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(border)
        .title(" Lists ");
    let inner = lists.inner(lists_area);
    frame.render_widget(lists, lists_area);
    let mut y = inner.y;
    let mut shown = false;
    for (index, row) in app.rows.iter().enumerate() {
        if y >= inner.y + inner.height {
            break;
        }
        match row {
            ListRow::Favorite { .. } | ListRow::Smart(_) => continue,
            ListRow::Account(address) => {
                let line = Line::from(format!(" {address}")).style(Style::new().dim());
                frame.render_widget(Paragraph::new(line), Rect::new(inner.x, y, inner.width, 1));
                y += 1;
            }
            ListRow::List { list, .. } => {
                shown = true;
                let selected = index == app.selected_list && focus;
                let mut text = format!("  {}", list.name);
                if list.open > 0 {
                    text.push_str(&format!(" ({})", list.open));
                }
                if list.is_shared {
                    text.push_str("  ⇄");
                }
                let line = Line::from(text);
                let line = if selected {
                    line.style(Style::new().add_modifier(Modifier::REVERSED))
                } else {
                    line
                };
                frame.render_widget(Paragraph::new(line), Rect::new(inner.x, y, inner.width, 1));
                y += 1;
            }
        }
    }
    if !shown && inner.height > 0 {
        frame.render_widget(
            Paragraph::new("  a makes a list").style(Style::new().dim()),
            Rect::new(inner.x, y, inner.width, 1),
        );
    }
}

fn draw_tasks(frame: &mut Frame, area: Rect, app: &AppState) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let focus = app.focus == Pane::Tasks;
    let title = match app.mark_list_summary() {
        Some(marks) => format!(" {}{marks}", app.selected_list_name()),
        None => format!(" {} ", app.selected_list_name()),
    };
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(if focus {
            Style::new().fg(Color::Cyan)
        } else {
            Style::new()
        })
        .title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let lines = task_lines(app, inner.width);
    let selected = app.selected_task;
    let position = lines
        .iter()
        .position(|line| line.task == Some(selected))
        .unwrap_or(0);
    let height = usize::from(inner.height);
    let top = position.saturating_sub(height.saturating_sub(1));
    for (offset, line) in lines.iter().skip(top).take(height).enumerate() {
        let y = inner.y + offset as u16;
        frame.render_widget(
            Paragraph::new(line.line.clone()),
            Rect::new(inner.x, y, inner.width, 1),
        );
    }
    if lines.is_empty() && inner.height > 0 {
        frame.render_widget(
            Paragraph::new("  No tasks. a adds one").style(Style::new().dim()),
            Rect::new(inner.x, inner.y, inner.width, 1),
        );
    }
    if app.searching()
        && app
            .search
            .as_ref()
            .is_some_and(|search| !search.query().trim().is_empty())
        && lines.is_empty()
    {
        frame.render_widget(
            Paragraph::new("  Nothing matches").style(Style::new().dim()),
            Rect::new(inner.x, inner.y, inner.width, 1),
        );
    }
}

struct TaskLine {
    task: Option<usize>,
    line: Line<'static>,
}

fn task_lines(app: &AppState, width: u16) -> Vec<TaskLine> {
    let today = app.today();
    let mut lines = Vec::new();
    let mut last: Option<Group> = None;
    let mut ordinal = 0;
    for row in &app.tasks {
        if app.completed_hidden && !row.task.status.is_open() {
            continue;
        }
        let group = groups::group_of(&row.task, &app.view, &row.list_name, today);
        if !app.searching() && last.as_ref() != Some(&group) {
            if let Some(title) = group.title() {
                let count = app
                    .tasks
                    .iter()
                    .filter(|candidate| {
                        groups::group_of(&candidate.task, &app.view, &candidate.list_name, today)
                            == group
                    })
                    .count();
                let folded = app.collapsed_groups.contains(&group.key());
                let marker = if folded { "▸" } else { "▾" };
                lines.push(TaskLine {
                    task: None,
                    line: Line::from(format!(" {marker} {title} ({count})"))
                        .style(Style::new().fg(Color::DarkGray)),
                });
            }
            last = Some(group.clone());
        }
        if !app.searching() && app.collapsed_groups.contains(&group.key()) {
            continue;
        }
        let marked = app.marks.contains(&row.task.id);
        let selected = ordinal == app.selected_task && app.focus == Pane::Tasks;
        lines.push(TaskLine {
            task: Some(ordinal),
            line: task_line(row, selected, marked, width, today, app.searching()),
        });
        ordinal += 1;
    }
    lines
}

fn task_line(
    row: &TaskRow,
    selected: bool,
    marked: bool,
    width: u16,
    today: jiff::civil::Date,
    searching: bool,
) -> Line<'static> {
    let task = &row.task;
    let checkbox = if task.status.is_open() { "[ ]" } else { "[x]" };
    let marker = if selected {
        '>'
    } else if marked {
        '●'
    } else {
        ' '
    };
    let left = format!(" {marker} {checkbox} ");
    let mut right: Vec<Span> = Vec::new();
    if task.steps_total > 0 {
        right.push(Span::styled(
            format!("{}/{}", task.steps_done, task.steps_total),
            Style::new().fg(Color::DarkGray),
        ));
    }
    if task.recurring {
        right.push(Span::styled("↻", Style::new().fg(Color::DarkGray)));
    }
    if task.importance.is_starred() {
        right.push(Span::styled("★", Style::new().fg(Color::Yellow)));
    }
    if let Some(due) = task.due {
        let style = if due < today {
            Style::new().fg(Color::Red)
        } else {
            Style::new().fg(Color::DarkGray)
        };
        right.push(Span::styled(due_text(due, today), style));
    }
    if searching && !row.list_name.is_empty() {
        right.push(Span::styled(
            row.list_name.clone(),
            Style::new().fg(Color::DarkGray),
        ));
    }
    let right_text: Vec<String> = right.iter().map(|span| span.content.to_string()).collect();
    let right_width: usize = right_text
        .iter()
        .map(|text| text.chars().count())
        .sum::<usize>()
        + right_text.len().saturating_sub(1);
    let left_width = left.chars().count();
    let available = usize::from(width)
        .saturating_sub(left_width)
        .saturating_sub(right_width)
        .saturating_sub(1);
    let mut title = task.title.clone();
    if title.chars().count() > available {
        let keep = available.saturating_sub(1);
        title = title.chars().take(keep).collect::<String>() + "…";
    }
    let used = left_width + title.chars().count() + right_width;
    let padding = usize::from(width).saturating_sub(used);
    let mut spans = vec![Span::raw(left)];
    let title_style = if task.status.is_open() {
        Style::new()
    } else {
        Style::new()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::CROSSED_OUT)
    };
    spans.push(Span::styled(title, title_style));
    spans.push(Span::raw(" ".repeat(padding)));
    for (index, span) in right.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(" "));
        }
        spans.push(span);
    }
    Line::from(spans)
}

fn draw_details(frame: &mut Frame, area: Rect, app: &mut AppState) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let focus = app.focus == Pane::Details;
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(if focus {
            Style::new().fg(Color::Cyan)
        } else {
            Style::new()
        })
        .title(" Details ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let details = &mut app.details;
    details.follow(usize::from(inner.height));
    let lines = details.lines();
    if lines.is_empty() || lines == [""] {
        frame.render_widget(
            Paragraph::new("  No task selected").style(Style::new().dim()),
            inner,
        );
        return;
    }
    let scroll = details.scroll(usize::from(inner.height));
    let cursor_row = details.viewer.cursor().0;
    let selection = details.viewer.selection();
    for offset in 0..usize::from(inner.height) {
        let index = scroll + offset;
        let Some(text) = lines.get(index) else {
            break;
        };
        let width = numbers::gutter_width(lines.len());
        let label = numbers::label(index, cursor_row, width);
        let mut style = Style::new().fg(Color::DarkGray);
        if focus && index == cursor_row {
            style = Style::new().fg(Color::White);
        }
        if let Some((from, to)) = selection
            && index >= from.0
            && index <= to.0
        {
            style = style.bg(Color::DarkGray);
        }
        let line = Line::from(vec![
            Span::styled(label, Style::new().fg(Color::DarkGray)),
            Span::styled(text.clone(), style),
        ]);
        frame.render_widget(
            Paragraph::new(line),
            Rect::new(inner.x, inner.y + offset as u16, inner.width, 1),
        );
    }
}

fn draw_status(frame: &mut Frame, area: Rect, app: &mut AppState) {
    let mut spans = Vec::new();
    if let Some(update) = &app.update {
        spans.push(Span::styled(
            format!(" ↑{update} "),
            Style::new().fg(Color::Yellow),
        ));
    }
    if app.focus == Pane::Details || app.form.is_some() {
        let mode = app
            .form
            .as_ref()
            .map_or_else(|| app.details.viewer.mode(), |form| form.notes.vim.mode);
        spans.push(Span::styled(
            format!(" {} ", mode.label()),
            Style::new().fg(Color::Black).bg(Color::Cyan),
        ));
    }
    if app.search.as_ref().is_some_and(|search| search.typing()) {
        let query = app
            .search
            .as_ref()
            .map_or("", |search| search.query())
            .to_owned();
        spans.push(Span::raw(format!("  /{query}")));
    } else if let Some(crate::tui::app::Status::Error(text)) = &app.status {
        spans.push(Span::styled(
            format!("  {text}"),
            Style::new().fg(Color::Red),
        ));
    } else if let Some(crate::tui::app::Status::Info(text)) = &app.status {
        spans.push(Span::raw(format!("  {text}")));
    } else {
        spans.push(Span::raw(help::hint(app.focus)));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
    if app.cursor() == crate::tui::screen::Cursor::Bar {
        let x = if app.search.as_ref().is_some_and(|search| search.typing()) {
            area.x
                + 3
                + app
                    .search
                    .as_ref()
                    .map_or(0, |search| search.input.cursor()) as u16
        } else {
            area.x
        };
        frame.set_cursor_position((x, area.y));
    }
}

fn draw_dialog(frame: &mut Frame, area: Rect, dialog: &Dialog, app: &AppState) {
    match dialog {
        Dialog::Help => draw_help(frame, area, app),
        Dialog::Confirm(confirm) => {
            let rect = centered(area, 62, 7);
            frame.render_widget(Clear, rect);
            let block = Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(Color::Red))
                .title(" Delete ");
            let text = vec![
                Line::from(""),
                Line::from(format!("  {}", confirm.prompt)),
                Line::from(""),
                Line::from("  y / Enter delete   Esc or any other key cancel")
                    .style(Style::new().dim()),
            ];
            frame.render_widget(Paragraph::new(text).block(block), rect);
        }
        Dialog::Name(prompt) => {
            let rect = centered(area, 52, 7);
            frame.render_widget(Clear, rect);
            let block = Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(Color::Yellow))
                .title(format!(" {} ", prompt.title));
            let (text, _) = prompt.input.view(40);
            let lines = vec![
                Line::from(""),
                Line::from(format!("  {}{text}", prompt.label)),
                Line::from(""),
                Line::from("  Enter save   Esc cancel").style(Style::new().dim()),
            ];
            frame.render_widget(Paragraph::new(lines).block(block), rect);
        }
        Dialog::Due(prompt) => {
            let rect = centered(area, 52, 8);
            frame.render_widget(Clear, rect);
            let block = Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(Color::Yellow))
                .title(" Due ");
            let (text, _) = prompt.input.view(40);
            let mut lines = vec![Line::from(""), Line::from(format!("  Due:  {text}"))];
            if let Some(error) = &prompt.error {
                lines.push(Line::from(format!("  {error}")).style(Style::new().fg(Color::Red)));
            } else {
                lines.push(Line::from(""));
            }
            lines.push(Line::from("  Enter save   Esc cancel").style(Style::new().dim()));
            frame.render_widget(Paragraph::new(lines).block(block), rect);
        }
        Dialog::Move(picker) => draw_move(frame, area, picker),
        Dialog::Accounts(accounts) => {
            let rect = centered(area, 60, 12);
            frame.render_widget(Clear, rect);
            let block = Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(Color::Cyan))
                .title(" Accounts ");
            let mut lines = vec![Line::from("")];
            for (index, address) in accounts.addresses.iter().enumerate() {
                let selected = index == accounts.selected;
                let style = if selected {
                    Style::new().add_modifier(Modifier::REVERSED)
                } else {
                    Style::new()
                };
                lines.push(Line::from(format!("  {address}")).style(style));
            }
            lines.push(Line::from(""));
            lines.push(
                Line::from("  Add or remove accounts with `todo login` and `todo logout`.")
                    .style(Style::new().dim()),
            );
            lines.push(Line::from("  Esc closes").style(Style::new().dim()));
            frame.render_widget(Paragraph::new(lines).block(block), rect);
        }
    }
}

fn draw_help(frame: &mut Frame, area: Rect, app: &AppState) {
    let width = area.width.min(76);
    let height = area.height.saturating_sub(2);
    let rect = centered(area, width, height.min(34));
    frame.render_widget(Clear, rect);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(Color::Cyan))
        .title(" Keys ");
    let mut lines = Vec::new();
    for (title, rows) in help::sections(app) {
        lines.push(Line::from(format!(" {title} ")).style(Style::new().fg(Color::Cyan).bold()));
        for row in rows {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<18}", row.key), Style::new().fg(Color::Cyan)),
                Span::raw(row.what),
            ]));
        }
        lines.push(Line::from(""));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        rect,
    );
}

fn draw_move(frame: &mut Frame, area: Rect, picker: &MovePicker) {
    let rect = centered(area, 52, 16);
    frame.render_widget(Clear, rect);
    let count = picker.tasks.len();
    let title = if count == 1 {
        "Move 1 task to".to_owned()
    } else {
        format!("Move {count} tasks to")
    };
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(Color::Yellow))
        .title(format!(" {title} "));
    let (filter, _) = picker.filter.filter.view(38);
    let mut lines = vec![Line::from(format!("  List:  {filter}")), Line::from("")];
    for (index, (_, name)) in picker.shown().into_iter().enumerate() {
        let selected = index == picker.filter.selected;
        let style = if selected {
            Style::new().add_modifier(Modifier::REVERSED)
        } else {
            Style::new()
        };
        lines.push(Line::from(format!("  {name}")).style(style));
    }
    lines.push(Line::from(""));
    lines.push(Line::from("  Enter move   Esc cancel").style(Style::new().dim()));
    frame.render_widget(Paragraph::new(lines).block(block), rect);
}

fn draw_form(frame: &mut Frame, area: Rect, form: &TaskForm) {
    let rect = area;
    frame.render_widget(Clear, rect);
    let block = Block::bordered()
        .border_type(BorderType::Thick)
        .border_style(Style::new().fg(Color::Yellow))
        .title(" Edit task ");
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
    if inner.height < 8 {
        return;
    }
    let [fields, notes] =
        Layout::vertical([Constraint::Length(6), Constraint::Min(1)]).areas(inner);
    let active = form.field();
    let style = |field: Field| {
        if field == active {
            Style::new().fg(Color::Yellow)
        } else {
            Style::new()
        }
    };
    let mut y = fields.y;
    let title = form.title.text();
    frame.render_widget(
        Paragraph::new(Line::from(format!("  Title:   {title}"))).style(style(Field::Title)),
        Rect::new(fields.x, y, fields.width, 1),
    );
    y += 1;
    let due = form.due.text();
    let error = form.due_error.as_deref().unwrap_or("");
    frame.render_widget(
        Paragraph::new(Line::from(format!("  Due:     {due}  {error}"))).style(style(Field::Due)),
        Rect::new(fields.x, y, fields.width, 1),
    );
    y += 1;
    let remind = form.remind.text();
    let error = form.remind_error.as_deref().unwrap_or("");
    frame.render_widget(
        Paragraph::new(Line::from(format!("  Remind:  {remind}  {error}")))
            .style(style(Field::Remind)),
        Rect::new(fields.x, y, fields.width, 1),
    );
    y += 1;
    let starred = if form.starred { "[x]" } else { "[ ]" };
    frame.render_widget(
        Paragraph::new(Line::from(format!("  Starred: {starred}"))).style(style(Field::Starred)),
        Rect::new(fields.x, y, fields.width, 1),
    );
    y += 1;
    frame.render_widget(
        Paragraph::new(Line::from("  ── Notes ")).style(Style::new().dim()),
        Rect::new(fields.x, y, fields.width, 1),
    );
    let notes_block = Block::new().style(style(Field::Notes));
    let notes_inner = notes_block.inner(notes);
    frame.render_widget(notes_block, notes);
    let cursor = form.notes_cursor();
    let scroll = form.notes_scroll(usize::from(notes_inner.height));
    let width = numbers::gutter_width(form.notes_lines().len());
    for (offset, text) in form
        .notes_lines()
        .iter()
        .skip(scroll)
        .take(usize::from(notes_inner.height))
        .enumerate()
    {
        let index = scroll + offset;
        let label = numbers::label(index, cursor.0, width);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(label, Style::new().fg(Color::DarkGray)),
                Span::raw(text.clone()),
            ])),
            Rect::new(
                notes_inner.x,
                notes_inner.y + offset as u16,
                notes_inner.width,
                1,
            ),
        );
    }
    let hint = "  Tab next · Ctrl-S save · Esc close";
    frame.render_widget(
        Paragraph::new(hint).style(Style::new().dim()),
        Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1),
    );
    if form.field() == Field::Notes {
        let x = notes_inner.x
            + width
            + u16::try_from(cursor.1)
                .unwrap_or(0)
                .min(notes_inner.width.saturating_sub(1));
        let y = notes_inner.y + u16::try_from(cursor.0.saturating_sub(scroll)).unwrap_or(0);
        frame.set_cursor_position((x, y));
    }
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    )
}
