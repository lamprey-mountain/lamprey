use async_trait::async_trait;
use common::v1::types::{
    room_analytics::{
        RoomAnalyticsChannel, RoomAnalyticsChannelParams, RoomAnalyticsInvites,
        RoomAnalyticsMembersCount, RoomAnalyticsMembersJoin,
        RoomAnalyticsMembersLeave, RoomAnalyticsOverview, RoomAnalyticsParams,
    },
    RoomId,
};

use crate::data::DataRoomAnalytics;
use crate::error::Result;

use super::Postgres;

#[async_trait]
impl DataRoomAnalytics for Postgres {
    async fn room_analytics_members_count(
        &self,
        _room_id: RoomId,
        _q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsMembersCount>> {
        todo!()
    }

    async fn room_analytics_members_join(
        &self,
        _room_id: RoomId,
        _q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsMembersJoin>> {
        todo!()
    }

    async fn room_analytics_members_leave(
        &self,
        _room_id: RoomId,
        _q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsMembersLeave>> {
        todo!()
    }

    async fn room_analytics_channels(
        &self,
        _room_id: RoomId,
        _q: RoomAnalyticsParams,
        _q2: RoomAnalyticsChannelParams,
    ) -> Result<Vec<RoomAnalyticsChannel>> {
        todo!()
    }

    async fn room_analytics_overview(
        &self,
        _room_id: RoomId,
        _q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsOverview>> {
        todo!()
    }

    async fn room_analytics_invites(
        &self,
        _room_id: RoomId,
        _q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsInvites>> {
        todo!()
    }
}
