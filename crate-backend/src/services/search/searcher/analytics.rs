use crate::services::search::schema::analytics::AnalyticsSchema;
use common::v1::types::room_analytics::{
    Aggregation, RoomAnalyticsChannel, RoomAnalyticsChannelParams, RoomAnalyticsInvites,
    RoomAnalyticsMembersCount, RoomAnalyticsMembersJoin, RoomAnalyticsMembersLeave,
    RoomAnalyticsOverview, RoomAnalyticsParams,
};
use common::v1::types::util::Time;
use common::v1::types::RoomId;
use lamprey_backend_core::prelude::*;
use serde_json::json;
use tantivy::aggregation::AggregationCollector;
use tantivy::query::{BooleanQuery, TermQuery};
use tantivy::schema::IndexRecordOption;
use tantivy::{IndexReader, Term};
use time::OffsetDateTime;

pub struct AnalyticsSearcher {
    reader: IndexReader,
    schema: AnalyticsSchema,
}

impl AnalyticsSearcher {
    pub fn new(reader: IndexReader, schema: AnalyticsSchema) -> Self {
        Self { reader, schema }
    }

    pub fn members_count(
        &self,
        room_id: RoomId,
        q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsMembersCount>> {
        let searcher = self.reader.searcher();

        let interval_str = match q.aggregate {
            Aggregation::Hourly => "1h",
            Aggregation::Daily => "1d",
            Aggregation::Weekly => "7d",
            Aggregation::Monthly => "30d",
        };

        let agg_req = json!({
            "timeline": {
                "date_histogram": {
                    "field": "created_at",
                    "fixed_interval": interval_str
                },
                "aggs": {
                    "kinds": {
                        "terms": {
                            "field": "event_kind"
                        },
                        "aggs": {
                            "total_value": {
                                "sum": {
                                    "field": "count"
                                }
                            }
                        }
                    }
                }
            }
        });

        let agg_req: tantivy::aggregation::agg_req::Aggregations =
            serde_json::from_value(agg_req).expect("Failed to parse hardcoded aggregation JSON");

        // build query
        let room_id_term = Term::from_field_text(self.schema.room_id, &room_id.to_string());
        let member_join_term = Term::from_field_text(self.schema.event_kind, "MemberJoin");
        let member_leave_term = Term::from_field_text(self.schema.event_kind, "MemberLeave");

        let kind_query = BooleanQuery::new(vec![
            (
                tantivy::query::Occur::Should,
                Box::new(TermQuery::new(member_join_term, IndexRecordOption::Basic)),
            ),
            (
                tantivy::query::Occur::Should,
                Box::new(TermQuery::new(member_leave_term, IndexRecordOption::Basic)),
            ),
        ]);

        let query = BooleanQuery::new(vec![
            (
                tantivy::query::Occur::Must,
                Box::new(TermQuery::new(room_id_term, IndexRecordOption::Basic)),
            ),
            (tantivy::query::Occur::Must, Box::new(kind_query)),
        ]);

        // run search
        let collector = AggregationCollector::from_aggs(agg_req, Default::default());
        let agg_res = searcher.search(&query, &collector)?;

        // HACK: use json formatting to bypass internal enum fuckery
        let res_json =
            serde_json::to_value(&agg_res).expect("Failed to serialize aggregation results");

        let mut results = vec![];
        let mut current_total: i64 = 0;

        if let Some(buckets) = res_json["timeline"]["buckets"].as_array() {
            for bucket in buckets {
                let mut delta: i64 = 0;

                let timestamp_ms = bucket["key"].as_i64().unwrap_or(0);

                if let Some(kinds) = bucket["kinds"]["buckets"].as_array() {
                    for kind_bucket in kinds {
                        let kind_name = kind_bucket["key"].as_str().unwrap_or("");
                        let sum_val =
                            kind_bucket["total_value"]["value"].as_f64().unwrap_or(0.0) as i64;

                        match kind_name {
                            "MemberJoin" => delta += sum_val,
                            "MemberLeave" => delta -= sum_val,
                            _ => {}
                        }
                    }
                }

                current_total += delta;

                let dt =
                    OffsetDateTime::from_unix_timestamp_nanos(timestamp_ms as i128 * 1_000_000)
                        .map_err(|e| Error::Internal(e.to_string()))?;
                let dt: Time = dt.into();

                results.push(RoomAnalyticsMembersCount {
                    bucket: dt.into(),
                    count: current_total.max(0) as u64,
                });
            }
        }

        if let Some(limit) = q.limit {
            results.truncate(limit as usize);
        }

        Ok(results)
    }

    pub fn members_join(
        &self,
        room_id: RoomId,
        q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsMembersJoin>> {
        todo!()
    }

    pub fn members_leave(
        &self,
        room_id: RoomId,
        q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsMembersLeave>> {
        todo!()
    }

    pub fn channels(
        &self,
        room_id: RoomId,
        q: RoomAnalyticsParams,
        channel_params: RoomAnalyticsChannelParams,
    ) -> Result<Vec<RoomAnalyticsChannel>> {
        todo!()
    }

    pub fn overview(
        &self,
        room_id: RoomId,
        q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsOverview>> {
        todo!()
    }

    pub fn invites(&self, room_id: RoomId, q: RoomAnalyticsParams) -> Result<RoomAnalyticsInvites> {
        todo!()
    }
}
