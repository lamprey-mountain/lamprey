CREATE INDEX message_embed_hosts_gin_idx ON message USING GIN (embed_hosts(embeds));
CREATE INDEX message_pinned_idx ON message (id) WHERE pinned IS NOT NULL and is_latest and deleted_at is null;
