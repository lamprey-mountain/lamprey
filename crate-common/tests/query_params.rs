use std::str::FromStr;

use lamprey_common::v1::types::{
    AuditLogEntryId, AuditLogFilter, ChannelId, PaginationDirection, PaginationQuery, query,
};

const CHANNEL_A: &str = "019b2d1a-fc40-7aa2-a5f3-9b24c2aea8cd";
const CHANNEL_B: &str = "019b2d1a-fc40-7aa2-a5f3-9b24c2aea8ce";

#[test]
fn pagination_and_filter_parse_from_same_query() {
    let query_str = format!("dir=b&limit=100&channel_id={CHANNEL_A}&type=RoomUpdate");

    let pagination: PaginationQuery<AuditLogEntryId> = query::from_str(&query_str).unwrap();
    assert_eq!(pagination.dir, Some(PaginationDirection::B));
    assert_eq!(pagination.limit, Some(100));
    assert_eq!(pagination.from, None);

    let filter: AuditLogFilter = query::from_str(&query_str).unwrap();
    assert_eq!(filter.channel_id, vec![ChannelId::from_str(CHANNEL_A).unwrap()]);
    assert_eq!(filter.ty, vec!["RoomUpdate".to_string()]);
    assert!(filter.user_id.is_empty());
    assert_eq!(filter.status, AuditLogFilter::default().status);
}

#[test]
fn repeated_keys_are_sequences() {
    let query_str =
        format!("channel_id={CHANNEL_A}&type=RoomUpdate&channel_id={CHANNEL_B}&type=RoomDelete");

    let filter: AuditLogFilter = query::from_str(&query_str).unwrap();
    assert_eq!(
        filter.channel_id,
        vec![
            ChannelId::from_str(CHANNEL_A).unwrap(),
            ChannelId::from_str(CHANNEL_B).unwrap(),
        ]
    );
    assert_eq!(filter.ty, vec!["RoomUpdate".to_string(), "RoomDelete".to_string()]);
}

#[test]
fn empty_query_uses_defaults() {
    let pagination: PaginationQuery<AuditLogEntryId> = query::from_str("").unwrap();
    assert_eq!(pagination.dir, None);

    let filter: AuditLogFilter = query::from_str("").unwrap();
    assert!(filter.channel_id.is_empty());
    assert_eq!(filter.status, AuditLogFilter::default().status);
}

#[test]
fn invalid_values_are_rejected() {
    let limit_err = query::from_str::<PaginationQuery<AuditLogEntryId>>("limit=lots").unwrap_err();
    assert!(!limit_err.to_string().is_empty());

    let channel_err = query::from_str::<AuditLogFilter>("channel_id=not-a-uuid").unwrap_err();
    assert!(!channel_err.to_string().is_empty());

    let dir_err = query::from_str::<PaginationQuery<AuditLogEntryId>>("dir=x").unwrap_err();
    assert!(!dir_err.to_string().is_empty());
}
