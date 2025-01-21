CREATE INDEX message_fts ON message USING GIN (to_tsvector('english', content));
