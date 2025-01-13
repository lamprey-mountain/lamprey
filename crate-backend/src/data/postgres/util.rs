use std::result::Result;

use uuid::Uuid;

use crate::{error::Error, types::{Identifier, PaginationDirection, PaginationQuery}};

#[derive(Debug)]
pub struct Pagination<I> {
    pub before: I,
    pub after: I,
    pub dir: PaginationDirection,
    pub limit: u16,
}

impl<I: Identifier> TryInto<Pagination<I>> for PaginationQuery<I> {
    type Error = Error;

    fn try_into(self) -> Result<Pagination<I>, Self::Error> {
        let limit = self.limit.unwrap_or(10);
        if limit > 100 {
            return Err(Error::TooBig);
        }
        let dir = self.dir.unwrap_or_default();
        let after = match dir {
            PaginationDirection::F => self.from,
            _ => self.to,
        };
        let after = after.unwrap_or(Uuid::nil().into());
        let before = match dir {
            PaginationDirection::F => self.to,
            _ => self.from,
        };
        let before = before.unwrap_or(Uuid::max().into());
        Ok(Pagination {
            before,
            after,
            dir,
            limit,
        })
    }
}
