use std::result::Result;

use types::PaginationKey;

use crate::{
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
