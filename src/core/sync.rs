//! Pulling a Microsoft To Do account's lists and tasks into the cache.

use crate::{
    model::{BackendError, ListId, TodoBackend},
    store::{Store, StoreError},
};

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error(transparent)]
    Backend(#[from] BackendError),
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// Fetches list changes and caches them. An expired delta link is dropped and
/// the lists are fetched whole once.
pub async fn sync_lists<B: TodoBackend>(backend: &B, store: &mut Store) -> Result<(), SyncError> {
    let link = store.lists_delta_link()?;
    let mut fresh = link.is_none();
    let page = match backend.lists_delta(link.as_deref()).await {
        Ok(page) => page,
        Err(BackendError::DeltaExpired) if link.is_some() => {
            store.set_lists_delta_link(None)?;
            fresh = true;
            backend.lists_delta(None).await?
        }
        Err(error) => return Err(error.into()),
    };
    store.upsert_lists(&page.upserted)?;
    store.remove_lists(&page.removed)?;
    if fresh {
        let keep: Vec<ListId> = store
            .lists()?
            .iter()
            .filter(|list| {
                page.upserted.iter().any(|new| new.id == list.id)
                    && !page.removed.contains(&list.id)
            })
            .map(|list| list.id.clone())
            .collect();
        store.retain_lists(&keep)?;
    }
    store.set_lists_delta_link(page.delta_link.as_deref())?;
    Ok(())
}

/// Fetches one list's task changes and caches them. An expired delta link is
/// dropped, that list's tasks are cleared and the list is fetched whole once.
pub async fn sync_tasks<B: TodoBackend>(
    backend: &B,
    store: &mut Store,
    list: &ListId,
) -> Result<(), SyncError> {
    let link = store.delta_link(list)?;
    let mut fresh = link.is_none();
    let page = match backend.tasks_delta(list, link.as_deref()).await {
        Ok(page) => page,
        Err(BackendError::DeltaExpired) if link.is_some() => {
            store.set_delta_link(list, None)?;
            store.clear_tasks_in(list)?;
            fresh = true;
            backend.tasks_delta(list, None).await?
        }
        Err(error) => return Err(error.into()),
    };
    if fresh {
        store.clear_tasks_in(list)?;
    }
    store.upsert_tasks(&page.upserted)?;
    store.remove_tasks(&page.removed)?;
    store.set_delta_link(list, page.delta_link.as_deref())?;
    Ok(())
}
