CREATE TABLE webhook (
    id UUID PRIMARY KEY REFERENCES usr(id) ON DELETE CASCADE,
    token TEXT NOT NULL,
    channel_id UUID NOT NULL REFERENCES channel(id) ON DELETE CASCADE,
    creator_id UUID REFERENCES usr(id) ON DELETE SET NULL
);

CREATE INDEX ON webhook (channel_id);
