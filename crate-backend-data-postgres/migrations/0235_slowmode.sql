ALTER TABLE channel
ADD COLUMN slowmode_thread INTEGER DEFAULT NULL,
ADD COLUMN slowmode_message INTEGER DEFAULT NULL,
ADD COLUMN default_slowmode_message INTEGER DEFAULT NULL;

CREATE TABLE channel_slowmode_message (
    channel_id UUID NOT NULL,
    user_id UUID NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    PRIMARY KEY (channel_id, user_id)
);

CREATE TABLE channel_slowmode_thread (
    channel_id UUID NOT NULL,
    user_id UUID NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    PRIMARY KEY (channel_id, user_id)
);

CREATE INDEX idx_channel_slowmode_message_expires_at ON channel_slowmode_message (expires_at);
CREATE INDEX idx_channel_slowmode_thread_expires_at ON channel_slowmode_thread (expires_at);
