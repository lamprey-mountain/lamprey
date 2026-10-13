//! reminders, built on [`Store`].

use anyhow::Result;
use common::v1::types::misc::Time;
use common::v2::types::UserId;
use serde::{Deserialize, Serialize};

use crate::store::Store;

/// A reminder belonging to a single user.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reminder {
    pub id: u64,
    pub text: String,
    pub scheduled_at: Time,
}

/// namespace shared by every reminder key
const REMINDER_PREFIX: &str = "reminder";

/// key under which the next reminder id is stored
const NEXT_ID_KEY: &str = "reminder.next_id";

/// key for a single reminder: `reminder.<user>.<id>`
///
/// the id is stored as a string because the encoded key must never contain the
/// key separator byte
type ReminderKey = (String, UserId, String);

/// build the store key for a single reminder
fn reminder_key(user_id: UserId, id: u64) -> ReminderKey {
    (REMINDER_PREFIX.to_string(), user_id, id.to_string())
}

/// prefix matching every reminder belonging to `user_id`
fn user_prefix(user_id: UserId) -> (String, UserId) {
    (REMINDER_PREFIX.to_string(), user_id)
}

/// get the next reminder id
async fn next_id(store: &Store) -> Result<u64> {
    let current: Option<u64> = store.get(NEXT_ID_KEY.to_string()).await?;
    let next = current.unwrap_or(0) + 1;
    store.put(NEXT_ID_KEY.to_string(), &next).await?;
    Ok(next)
}

/// create a new reminder for `user_id` and return it
pub async fn add(
    store: &Store,
    user_id: UserId,
    text: String,
    scheduled_at: Time,
) -> Result<Reminder> {
    let reminder = Reminder {
        id: next_id(store).await?,
        text,
        scheduled_at,
    };
    store
        .put(reminder_key(user_id, reminder.id), &reminder)
        .await?;
    Ok(reminder)
}

/// remove a single reminder belonging to `user_id`
pub async fn remove(store: &Store, user_id: UserId, id: u64) -> Result<()> {
    store.delete(reminder_key(user_id, id)).await
}

/// remove every reminder belonging to `user_id`
pub async fn remove_all(store: &Store, user_id: UserId) -> Result<()> {
    store.delete_all(&user_prefix(user_id)).await
}

/// list every reminder belonging to `user_id`, ordered by scheduled time
pub async fn list(store: &Store, user_id: UserId) -> Result<Vec<Reminder>> {
    let mut reminders: Vec<(ReminderKey, Reminder)> = store.all(&user_prefix(user_id)).await?;
    reminders.sort_by_key(|(_, reminder)| reminder.scheduled_at);
    Ok(reminders
        .into_iter()
        .map(|(_, reminder)| reminder)
        .collect())
}
