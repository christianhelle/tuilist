//! Graph JSON shapes. Only the fields tuilist selects.

use jiff::Timestamp;
use serde::Deserialize;

/// One page of a Graph collection.
#[derive(Debug, Deserialize)]
pub struct Page<T> {
    pub value: Vec<T>,
    #[serde(rename = "@odata.nextLink")]
    pub next_link: Option<String>,
    #[serde(rename = "@odata.deltaLink")]
    pub delta_link: Option<String>,
}

/// A list, or the mark that a delta page deleted one.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoList {
    pub id: String,
    #[serde(rename = "@removed")]
    pub removed: Option<serde::de::IgnoredAny>,
    #[serde(default)]
    pub display_name: String,
    pub wellknown_list_name: Option<String>,
    pub is_owner: Option<bool>,
    pub is_shared: Option<bool>,
}

/// A task, or the mark that a delta page deleted one.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoTask {
    pub id: String,
    #[serde(rename = "@removed")]
    pub removed: Option<serde::de::IgnoredAny>,
    pub title: Option<String>,
    pub status: Option<String>,
    pub importance: Option<String>,
    pub body: Option<ItemBody>,
    pub due_date_time: Option<DateTimeTimeZone>,
    pub reminder_date_time: Option<DateTimeTimeZone>,
    pub is_reminder_on: Option<bool>,
    pub completed_date_time: Option<DateTimeTimeZone>,
    pub recurrence: Option<serde::de::IgnoredAny>,
    pub has_attachments: Option<bool>,
    pub created_date_time: Option<Timestamp>,
    pub last_modified_date_time: Option<Timestamp>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistItem {
    pub id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub is_checked: bool,
    pub created_date_time: Option<Timestamp>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemBody {
    #[serde(default)]
    pub content_type: String,
    #[serde(default)]
    pub content: String,
}

/// A wall-clock time and the zone it is in.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DateTimeTimeZone {
    pub date_time: String,
    pub time_zone: Option<String>,
}
