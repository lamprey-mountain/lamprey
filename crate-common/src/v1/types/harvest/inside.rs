//! the types used inside the generated archive
//!
//! the archive is one big sqlite3 database
//!
//! ## user
//!
//! - the user itself
//! - all user-specific data
//! - all room data created by the user
//! - all applications and users
//! - all dm channels
//!
//! ## room
//!
//! - the room itself
//! - all data in the room

use crate::v1::types::harvest::Harvest;

/// the harvest metadata
#[derive(Debug)]
pub struct HarvestMetadata {
    pub harvest: Harvest,
}

#[derive(Debug)]
pub enum ResourceType {
    User,
    Room,
    // etc
}

/*
CREATE TABLE metadata (
    data JSONB NOT NULL  -- the metadata of this harvest
);

-- option 1: global resources
CREATE TABLE resource (
    id TEXT PRIMARY KEY, -- the id of this resource
    type TEXT NOT NULL,  -- the type of this resource
    data JSONB NOT NULL  -- the json data of this resource
);

CREATE INDEX resource_type ON resource (type);
CREATE INDEX resource_type_id ON resource (type, id);

-- option 2: table per resource location

CREATE TABLE resource_global (
    id TEXT PRIMARY KEY, -- the id of this resource
    type TEXT NOT NULL,  -- the type of this resource
    data JSONB NOT NULL  -- the json data of this resource
);

CREATE TABLE resource_room (
    id TEXT PRIMARY KEY,   -- the id of this resource
    room_id TEXT NOT NULL, -- the room id of the resource
    type TEXT NOT NULL,    -- the type of this resource
    data JSONB NOT NULL    -- the json data of this resource
);

CREATE TABLE resource_channel (
    id TEXT PRIMARY KEY,      -- the id of this resource
    room_id TEXT,             -- the room id of the resource
    channel_id TEXT NOT NULL, -- the channel id of the resource
    type TEXT NOT NULL,       -- the type of this resource
    data JSONB NOT NULL       -- the json data of this resource
);

-- option 3: table per resource
CREATE TABLE message (
    id TEXT PRIMARY KEY,      -- the id of the message
    room_id TEXT,             -- the room id of the message
    channel_id TEXT NOT NULL, -- the channel id of the message
    data JSONB NOT NULL       -- the json data of this resource
);

-- etc?
*/

// TODO: write to sqlite3 database, stream to s3?
