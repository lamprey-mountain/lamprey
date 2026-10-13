//! storage for the bot

use anyhow::Result;
use serde::{Serialize, de::DeserializeOwned};
use sqlx::{Row, SqlitePool, sqlite::SqliteConnectOptions};
use std::str::FromStr;

mod key;

pub use key::Key;

use crate::store::key::KEY_SEP;

/// a basic document database/key value store, backed by sqlite3
#[derive(Clone, Debug)]
pub struct Store {
    pool: SqlitePool,
}

impl Store {
    /// open a store at `path`, creating if it doesnt exist
    pub async fn open(path: &str) -> Result<Self> {
        let opts = SqliteConnectOptions::from_str(path)?.create_if_missing(true);
        let pool = SqlitePool::connect_with(opts).await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS things (
                key  BLOB PRIMARY KEY NOT NULL,
                data JSONB NOT NULL
            ) WITHOUT ROWID;",
        )
        .execute(&pool)
        .await?;
        Ok(Self { pool })
    }

    pub async fn put<K: Key, V: Serialize>(&self, key: K, value: &V) -> Result<()> {
        let key_bytes = key.to_bytes();
        let data = serde_json::to_vec(value)?;
        sqlx::query("INSERT OR REPLACE INTO things (key, data) VALUES (?, jsonb(?))")
            .bind(key_bytes)
            .bind(data)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get<K: Key, V: DeserializeOwned>(&self, key: K) -> Result<Option<V>> {
        let key_bytes = key.to_bytes();
        let row = sqlx::query("SELECT json(data) as data FROM things WHERE key = ?")
            .bind(key_bytes)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => {
                let data: String = row.try_get("data")?;
                Ok(Some(serde_json::from_str(&data)?))
            }
            None => Ok(None),
        }
    }

    pub async fn all<P: Key, K: Key, V: DeserializeOwned>(
        &self,
        prefix: &P,
    ) -> Result<Vec<(K, V)>> {
        let (start, end) = scan_bounds(prefix);
        let rows = sqlx::query(
            "SELECT key, json(data) AS data FROM things
             WHERE key >= ? AND (? IS NULL OR key < ?)
             ORDER BY key ASC",
        )
        .bind(&start)
        .bind(&end)
        .bind(&end)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let k = K::from_bytes(&row.try_get::<Vec<u8>, _>("key")?)?;
                let v = serde_json::from_str(&row.try_get::<String, _>("data")?)?;
                Ok((k, v))
            })
            .collect()
    }

    pub async fn delete<K: Key>(&self, key: K) -> Result<()> {
        let key_bytes = key.to_bytes();
        sqlx::query("DELETE FROM things WHERE key = ?")
            .bind(key_bytes)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_all<P: Key>(&self, prefix: &P) -> Result<()> {
        let (start, end) = scan_bounds(prefix);
        sqlx::query("DELETE FROM things WHERE key >= ? AND (? IS NULL OR key < ?)")
            .bind(&start)
            .bind(&end)
            .bind(&end)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

/// Half-open range [start, end) covering every key that starts with `prefix`.
/// `end` is None when the prefix is all 0xFF bytes (scan to the end of the table).
fn prefix_range(prefix: &[u8]) -> (Vec<u8>, Option<Vec<u8>>) {
    let start = prefix.to_vec();
    let mut end = start.clone();
    while let Some(last) = end.last_mut() {
        if *last == 0xFF {
            end.pop();
        } else {
            *last += 1;
            return (start, Some(end));
        }
    }
    (start, None)
}

/// Bounds for "everything under this prefix segment".
fn scan_bounds<P: Key>(prefix: &P) -> (Vec<u8>, Option<Vec<u8>>) {
    let mut p = prefix.to_bytes();
    p.push(KEY_SEP);
    prefix_range(&p)
}
