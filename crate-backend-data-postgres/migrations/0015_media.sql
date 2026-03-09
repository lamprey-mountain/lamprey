CREATE TABLE media (
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

CREATE TABLE media_link (
    media_id UUID,
    target_id UUID,
    link_type INT NOT NULL,
    deleted_at BIGINT,
    PRIMARY KEY (media_id, target_id, link_type)
);
