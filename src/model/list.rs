//! Task lists: the containers Microsoft To Do groups tasks into.

use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ListId(pub String);

/// The lists Microsoft gives every account that cannot be renamed or deleted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WellKnown {
    DefaultList,
    FlaggedEmails,
}

impl WellKnown {
    /// The value Graph uses for this well-known list.
    pub fn as_str(self) -> &'static str {
        match self {
            WellKnown::DefaultList => "defaultList",
            WellKnown::FlaggedEmails => "flaggedEmails",
        }
    }

    /// Reads the value Graph uses, ignoring lists that are not well known.
    pub fn from_graph(value: &str) -> Option<Self> {
        match value {
            "defaultList" => Some(WellKnown::DefaultList),
            "flaggedEmails" => Some(WellKnown::FlaggedEmails),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskList {
    pub id: ListId,
    pub name: String,
    pub well_known: Option<WellKnown>,
    pub is_owner: bool,
    pub is_shared: bool,
    /// Open tasks counted by the store; not part of what Graph returns.
    pub open: u32,
}

/// Puts the default list first, then other lists by name ignoring case, then
/// `Flagged email` last.
pub fn sort_lists(lists: &mut [TaskList]) {
    lists.sort_by(compare_lists);
}

fn compare_lists(a: &TaskList, b: &TaskList) -> Ordering {
    rank(a).cmp(&rank(b)).then_with(|| {
        let name = a.name.to_lowercase().cmp(&b.name.to_lowercase());
        name.then_with(|| a.id.cmp(&b.id))
    })
}

fn rank(list: &TaskList) -> u8 {
    match list.well_known {
        Some(WellKnown::DefaultList) => 0,
        Some(WellKnown::FlaggedEmails) => 2,
        None => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(id: &str, name: &str, well_known: Option<WellKnown>) -> TaskList {
        TaskList {
            id: ListId(id.to_owned()),
            name: name.to_owned(),
            well_known,
            is_owner: true,
            is_shared: false,
            open: 0,
        }
    }

    #[test]
    fn lists_put_the_default_first_and_flagged_email_last() {
        let mut lists = vec![
            list("3", "Flagged email", Some(WellKnown::FlaggedEmails)),
            list("2", "Work", None),
            list("1", "Tasks", Some(WellKnown::DefaultList)),
            list("4", "Groceries", None),
        ];

        sort_lists(&mut lists);

        let names: Vec<&str> = lists.iter().map(|list| list.name.as_str()).collect();
        assert_eq!(names, ["Tasks", "Groceries", "Work", "Flagged email"]);
    }

    #[test]
    fn lists_ignore_case_when_ordering_by_name() {
        let mut lists = vec![list("2", "beta", None), list("1", "Alpha", None)];

        sort_lists(&mut lists);

        let names: Vec<&str> = lists.iter().map(|list| list.name.as_str()).collect();
        assert_eq!(names, ["Alpha", "beta"]);
    }
}
