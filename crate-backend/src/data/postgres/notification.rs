#![allow(unused)]

use async_trait::async_trait;
use common::v1::types::notifications::{Notification, NotificationReason};
use common::v1::types::{
    MessageVerId, NotificationId, PaginationDirection, PaginationQuery, PaginationResponse, Thread,
};
use sqlx::{query, query_file_as, query_scalar, Acquire};

use crate::error::Result;
use crate::gen_paginate;
use crate::types::{DbThread, DbThreadType, ThreadId, UserId};

use crate::data::DataNotification;

use super::{util::Pagination, Postgres};

fn notif_reason_str(r: NotificationReason) -> &'static str {
    match r {
        NotificationReason::Mention => "Mention",
        NotificationReason::MentionBulk => "MentionBulk",
        NotificationReason::Reply => "Reply",
        NotificationReason::Reminder => "Reminder",
    }
}

fn notif_reason_parse(s: &str) -> NotificationReason {
    match s {
        "Mention" => NotificationReason::Mention,
        "MentionBulk" => NotificationReason::MentionBulk,
        "Reply" => NotificationReason::Reply,
        "Reminder" => NotificationReason::Reminder,
        _ => panic!("invalid data in db"),
    }
}

#[async_trait]
impl DataNotification for Postgres {
    async fn notification_add(&self, user_id: UserId, notif: Notification) -> Result<()> {
        todo!()
    }

    async fn notification_delete(&self, user_id: UserId, notif: NotificationId) -> Result<()> {
        todo!()
    }

    async fn notification_list(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<NotificationId>,
    ) -> Result<PaginationResponse<Notification>> {
        todo!()
    }
}
