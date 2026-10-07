//! flume control stream
//!
//! each flume stream controls a flume

// TODO: deprecate the http flume api

pub struct Initial {
    // do i create or connect to a flume here?
    // maybe http flume_create endpoint creates a new disconnected flume and the client has to "resume"/"reconnect" to it?
    // POST /channel/{channel_id}/flume -- do i keep any other endpoints?

    // channel_id, message_id
}

pub enum Command {
    // ping -- keepalive
    // delta -- incremental update
    // commit -- final
}

pub enum Event {
    // do i send anything here? or do i send response data via the main stream?
    // ack -- ack a delta (maybe allow acking multiple?)
}
