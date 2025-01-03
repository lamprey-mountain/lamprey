-- replace users with only members (ie. make users less "persistent") and accounts?
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    parent_id TEXT,
    name TEXT,
    description TEXT,
    avatar_url TEXT,
    email TEXT,
    status TEXT,
    is_bot INT,
    is_alias INT,
    is_system INT,
    can_fork INT,
    discord_id TEXT,
    FOREIGN KEY (parent_id) REFERENCES users(id)
);
