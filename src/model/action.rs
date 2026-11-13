//! What a key asks the UI to do, and what the engine says back.

use crate::model::ListId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    FocusNextPane,
    FocusPrevPane,
    SelectNext,
    SelectPrev,
    PageDown,
    PageUp,
    HalfPageDown,
    HalfPageUp,
    Top,
    Bottom,
    Open,
    Expand,
    Collapse,
    Quit,
    ToggleZoom,
    ToggleLists,
    ToggleDetailsPlace,
    ToggleMark,
    MarkAll,
    ClearMarks,
    Refresh,
    Help,
    Search,
    Update,
    ManageAccounts,
    /// `a`: a list in the lists pane, a task in the tasks pane, a step in
    /// the details.
    Add,
    /// `F2`.
    Rename,
    /// `d`, `D`.
    Delete,
    /// `x`.
    ToggleComplete,
    /// `*`.
    ToggleStar,
    /// `e`, `i`.
    Edit,
    /// `t`.
    DueToday,
    /// `T`.
    AskDue,
    /// `s`.
    AddStep,
    /// `m`.
    Move,
    /// `y`.
    Copy,
    /// `c`.
    ToggleCompletedShown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    ListsSynced,
    ListSynced(ListId),
    SyncFinished,
    /// The store changed; reload what is shown.
    TasksChanged(Vec<ListId>),
    StepsLoaded(crate::model::TaskId),
    /// Already reverted in the store.
    Failed {
        what: String,
        reason: String,
    },
    /// Refreshing the sign-in failed.
    SignedOut,
}
