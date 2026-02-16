CREATE TABLE tag (
    id UUID PRIMARY KEY,
    version_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    color TEXT,
    is_archived BOOLEAN NOT NULL,
    is_restricted BOOLEAN NOT NULL,
    FOREIGN KEY (channel_id) REFERENCES channel(id) ON DELETE CASCADE
);

CREATE TABLE channel_tag (
    channel_id UUID NOT NULL,
    tag_id UUID NOT NULL,
    PRIMARY KEY (channel_id, tag_id),
    FOREIGN KEY (channel_id) REFERENCES channel(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tag(id) ON DELETE CASCADE
);
