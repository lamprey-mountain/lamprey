use tantivy::schema::{
    self, IndexRecordOption, Schema, SchemaBuilder, TextFieldIndexing, TextOptions, FAST, INDEXED,
    STORED, STRING,
};

/// tantivy schema for `Message`s
#[derive(Debug, Clone)]
pub struct MessageSchema {
    pub schema: Schema,
    pub author_id: schema::Field,
    pub channel_id: schema::Field,
    pub content: schema::Field,
    pub created_at: schema::Field,
    pub has_attachment: schema::Field,
    pub has_audio: schema::Field,
    pub has_embed: schema::Field,
    pub has_image: schema::Field,
    pub has_link: schema::Field,
    pub has_thread: schema::Field,
    pub has_video: schema::Field,
    pub id: schema::Field,
    pub is_pinned: schema::Field,
    pub is_reply: schema::Field,
    pub link_hostname: schema::Field,
    pub mentions_everyone: schema::Field,
    pub mentions_role: schema::Field,
    pub mentions_user: schema::Field,
    pub room_id: schema::Field,
}

/// tantivy schema for `Channel`s
#[derive(Debug, Clone)]
pub struct ChannelSchema {
    // TODO
}

impl Default for MessageSchema {
    fn default() -> Self {
        let mut sb = SchemaBuilder::new();

        let text_options = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_index_option(IndexRecordOption::WithFreqsAndPositions),
            )
            .set_stored();

        let author_id = sb.add_text_field("author_id", STRING);
        let channel_id = sb.add_text_field("channel_id", STRING | FAST);
        let content = sb.add_text_field("content", text_options);
        let created_at = sb.add_date_field("created_at", INDEXED | STORED);
        let has_attachment = sb.add_bool_field("has_attachment", INDEXED);
        let has_audio = sb.add_bool_field("has_audio", INDEXED);
        let has_embed = sb.add_bool_field("has_embed", INDEXED);
        let has_image = sb.add_bool_field("has_image", INDEXED);
        let has_link = sb.add_bool_field("has_link", INDEXED);
        let has_thread = sb.add_bool_field("has_thread", INDEXED);
        let has_video = sb.add_bool_field("has_video", INDEXED);
        let id = sb.add_text_field("id", STRING | FAST | STORED);
        let is_pinned = sb.add_bool_field("is_pinned", INDEXED);
        let is_reply = sb.add_bool_field("is_reply", INDEXED);
        let link_hostname = sb.add_text_field("link_hostname", STRING);
        let mentions_everyone = sb.add_bool_field("mentions_everyone", INDEXED);
        let mentions_role = sb.add_text_field("mentions_role", STRING);
        let mentions_user = sb.add_text_field("mentions_user", STRING);
        let room_id = sb.add_text_field("room_id", STRING | FAST);

        let schema = sb.build();

        Self {
            schema,
            author_id,
            channel_id,
            content,
            created_at,
            has_attachment,
            has_audio,
            has_embed,
            has_image,
            has_link,
            has_thread,
            has_video,
            id,
            is_pinned,
            is_reply,
            link_hostname,
            mentions_everyone,
            mentions_role,
            mentions_user,
            room_id,
        }
    }
}
