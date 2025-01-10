-- replace users with only members (ie. make users less "persistent") and accounts?
CREATE TABLE IF NOT EXISTS usr (
    id UUID PRIMARY KEY,
    parent_id UUID,
    name TEXT,
    description TEXT,
    avatar_url TEXT,
    email TEXT,
    status TEXT,
    is_bot BOOL NOT NULL,
    is_alias BOOL NOT NULL,
    is_system BOOL NOT NULL,
    can_fork BOOL NOT NULL,
    discord_id TEXT,
    deleted_at BIGINT,
    FOREIGN KEY (parent_id) REFERENCES usr(id)
);
