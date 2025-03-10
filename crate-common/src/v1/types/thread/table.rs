//! arbitrary data storage? like a spreadsheet or database table?
//! overengineering go BRRRRRRR

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{misc::Color, Media, ThreadId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadTypeTablePublic {
    // pub last_version_id: MessageVerId,
    pub table: Table,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Table {
    pub schema: Vec<Column>,
    // views? pub view: RowQuery,
    pub row_count: u64,
    // pub auth_list: Option<Formula>,
    // pub auth_view: Option<Formula>,
    // pub auth_create: Option<Formula>,
    // pub auth_update: Option<Formula>,
    // pub auth_delete: Option<Formula>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadTypeTablePrivate {
    pub is_unread: bool,
    // pub last_read_id: Option<MessageVerId>,
    pub mention_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Column {
    pub name: String,
    pub is_shown_by_default: bool,
    pub is_nullable: bool,
    pub is_indexed: bool,

    #[serde(rename = "type")]
    pub ty: ColumnType,

    #[serde(rename = "default")]
    pub ty_default: Option<ColumnDefault>,
    // pub auth_view: Option<Formula>,
    // pub auth_update: Option<Formula>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ColumnPatch {
    pub name: Option<String>,
    pub is_shown_by_default: Option<bool>,
    pub is_nullable: Option<bool>,
    pub is_indexed: Option<bool>,

    #[serde(rename = "type")]
    pub ty: ColumnType,

    #[serde(rename = "default")]
    pub ty_default: Option<ColumnDefault>,

    /// required if changing the type
    pub convert: Option<Formula>,
    // pub auth_view: Option<Option<Formula>>,
    // pub auth_update: Option<Option<Formula>>,
}

// NOTE: do i need different sizes (32 bit, 64 bit)? or always have full precision (bigint/bigfloat)?
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type", content = "type_info")]
pub enum ColumnType {
    Bool,
    Time,
    String,
    Url,
    Int,
    Uint,
    Float,
    Rel(ColumnTarget),
    Enum(Vec<ColumnEnumItem>),
    Thing(ColumnTypeThing),
    Formula(Formula),
    // the rabbit hole can go arbitrarily deep
    // DateOnly, TimeOnly
    // Duration, Measurement, Geometry
    // TextInline, TextBlock
    // Any, Array, Uuid
}

/// the type of cetahe things
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ColumnTypeThing {
    Media,
    Room,
    Thread,
    User,
    Message,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ColumnEnumItem {
    // unique, like column? or use ids?
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<Media>,
    pub color: Option<Color>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ColumnTarget {
    /// can only refer to threads of type Table
    pub table: Option<ThreadId>,
    pub column: String,
}

// this is absolutely a rabbit hole that im not sure i want to fall down yet...
/// a formula that calculates stuff
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Formula {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ColumnValue {
    Null,
    Bool(bool),
    Time,
    String,
    Int(i64),
    Uint(u64),
    Float(f64),
    Rel(Uuid),
    Enum(String),
    Thing(ColumnTypeThing),
    Url(Url),
    Formula(Box<ColumnValue>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ColumnDefault {
    // FIXME: eq
    // Const(ColumnValue),
    CurrentUser,
    CurrentDate,
}

/// a cetahe thing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ColumnValueThing {
    Media(crate::Media),
    Room(crate::Room),
    Thread(crate::Thread),
    User(crate::User),
    Message(crate::Message),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Row {
    pub id: Uuid,

    // maybe i should use created_at, updated_at, updated_by? (...or tell people
    // to use audit log)
    pub version_id: Uuid,

    #[serde(flatten)]
    pub fields: HashMap<String, ColumnValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RowQuery {
    #[serde(default)]
    pub columns: Vec<String>,

    pub filter: Formula,

    pub sort: Formula,

    /// skip calculating total
    #[serde(default)]
    pub skip_total: bool,
    // #[serde(flatten)]
    // pub pagination: crate::pagination::PaginationQuery<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RowPatch {
    #[serde(flatten)]
    pub fields: HashMap<String, Option<ColumnValue>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RowCreate {
    #[serde(flatten)]
    pub fields: HashMap<String, ColumnValue>,
}

/*
http api sketch

maybe make thread-specific apis use thread-specific paths for other threads too?

POST   /table/{id}/column
GET    /table/{id}/column/{id}
DELETE /table/{id}/column/{id}?force=[true|false]
PATCH  /table/{id}/column/{id}
POST   /table/{id}/row
GET    /table/{id}/row/{id}
DELETE /table/{id}/row/{id}
PATCH  /table/{id}/row/{id}
PATCH  /table/{id}/schema
POST   /table/{id}/truncate
POST   /table/{id}/query
GET    /table/{id}/export?format=[json|idk]
*/
