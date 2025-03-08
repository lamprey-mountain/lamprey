use std::result::Result;

use common::v1::types::{Media, PaginationKey};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    data::postgres::media::{DbMedia, DbMediaTrack},
    error::Error,
    types::{PaginationDirection, PaginationQuery},
};

#[derive(Debug)]
pub struct Pagination<K> {
    pub before: K,
    pub after: K,
    pub dir: PaginationDirection,
    pub limit: u16,
}

impl<K: PaginationKey> TryInto<Pagination<K>> for PaginationQuery<K> {
    type Error = Error;

    fn try_into(self) -> Result<Pagination<K>, Self::Error> {
        let limit = self.limit.unwrap_or(10);
        if limit > 100 {
            return Err(Error::TooBig);
        }
        let dir = self.dir.unwrap_or_default();
        let after = match dir {
            PaginationDirection::F => self.from.clone(),
            _ => self.to.clone(),
        };
        let after = after.unwrap_or(<K as PaginationKey>::min());
        let before = match dir {
            PaginationDirection::F => self.to,
            _ => self.from,
        };
        let before = before.unwrap_or(<K as PaginationKey>::max());
        Ok(Pagination {
            before,
            after,
            dir,
            limit,
        })
    }
}

#[macro_export]
macro_rules! gen_paginate {
    ($p:expr, $pool:expr, $qlist:expr, $qtotal:expr, $map:expr) => {{
        let mut conn = $pool.acquire().await?;
        let mut tx = conn.begin().await?;

        let items = $qlist.fetch_all(&mut *tx).await?;
        let total = $qtotal.fetch_one(&mut *tx).await?;
        let has_more = items.len() > $p.limit as usize;
        let mut items: Vec<_> = items
            .into_iter()
            .take($p.limit as usize)
            .map($map)
            .collect();
        if $p.dir == PaginationDirection::B {
            items.reverse();
        }

        // tx intentionally dropped to rollback here

        Ok(PaginationResponse {
            items,
            total: total.unwrap_or(0) as u64,
            has_more,
        })
    }};
    ($p:expr, $pool:expr, $qlist:expr, $qtotal:expr) => {
        gen_paginate!($p, $pool, $qlist, $qtotal, Into::into)
    };
}

pub fn media_from_db(value: Value) -> Media {
    #[derive(Deserialize)]
    struct Helper {
        #[serde(flatten)]
        media: DbMedia,
        tracks: Vec<DbMediaTrack>,
    }

    let row: Helper = serde_json::from_value(value).expect("invalid data in database!");
    row.media.upgrade(row.tracks).0
}
