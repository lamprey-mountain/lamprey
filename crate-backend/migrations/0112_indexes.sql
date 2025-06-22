CREATE INDEX idx_message_thread_latest 
ON message(thread_id, id DESC) 
WHERE is_latest AND deleted_at IS NULL;

DROP INDEX message_is_latest;
CREATE INDEX idx_message_latest_filtered 
ON message(thread_id, deleted_at, id) 
WHERE deleted_at IS NULL;

CREATE INDEX att_json_version_id ON att_json(version_id);
CREATE INDEX url_embed_json_version_id ON url_embed_json(version_id);
