CREATE TABLE IF NOT EXISTS media (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    url TEXT NOT NULL,
    source_url TEXT,
    thumbnail_url TEXT,
    filename TEXT,
    alt TEXT,
    size INT NOT NULL,
    mime TEXT NOT NULL,
    height INT,
    width INT,
    duration INT,
    FOREIGN KEY (user_id) REFERENCES usr(id)
);

CREATE TABLE IF NOT EXISTS media_link (
    media_id UUID,
    target_id UUID,
    deleted_at INT,
    PRIMARY KEY (media_id, target_id)
);
