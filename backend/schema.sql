CREATE TABLE rooms (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT
) STRICT;

CREATE TABLE role (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    permissions INT NOT NULL
) STRICT;

CREATE TABLE role_application (
    user_id INT,
    role_id INT,
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (role_id) REFERENCES roles(id),
    PRIMARY KEY (user_id, role_id)
) STRICT;

CREATE TABLE threads (
    id TEXT PRIMARY KEY,
    room_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    is_closed INT NOT NULL,
    is_locked INT NOT NULL,
    FOREIGN KEY (room_id) REFERENCES rooms(id)
) STRICT;

CREATE TABLE messages (
    id TEXT NOT NULL,
    thread_id TEXT NOT NULL,
    version_id TEXT PRIMARY KEY,
    ordering INT NOT NULL,
    content TEXT NOT NULL,
    metadata TEXT NOT NULL,
    reply TEXT,
    author_id TEXT NOT NULL,
    FOREIGN KEY (thread_id) REFERENCES threads(id),
    FOREIGN KEY (author_id) REFERENCES users(id)
    -- FOREIGN KEY (reply) REFERENCES messages(id)
) STRICT;

CREATE INDEX messages_message_id ON messages (id);

-- replace users with only members (ie. make users less "persistent") and accounts?
CREATE TABLE users (
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
) STRICT;

CREATE UNIQUE INDEX users_email ON users (email);

CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    token TEXT NOT NULL,
    name TEXT,
    status INT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
) STRICT;

CREATE TABLE auth (
    user_id TEXT NOT NULL,
    type TEXT NOT NULL,
    data ANY,
    PRIMARY KEY (user_id, type),
    FOREIGN KEY (user_id) REFERENCES users(id)
) STRICT;

CREATE TABLE room_members (
    id TEXT PRIMARY KEY,
    user_id TEXT,
    room_id TEXT,
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (room_id) REFERENCES rooms(id)
) STRICT;

CREATE TABLE thread_members (
    member_id TEXT NOT NULL,
    thread_id TEXT NOT NULL,
    FOREIGN KEY (member_id) REFERENCES members(id),
    FOREIGN KEY (thread_id) REFERENCES threads(id),
    PRIMARY KEY (member_id, thread_id)
) STRICT;

CREATE TABLE media (
    id TEXT PRIMARY KEY,
    purpose TEXT NOT NULL,
    size INT NOT NULL,
    uploader TEXT NOT NULL,
    deleted_at TEXT,
    FOREIGN KEY (uploader) REFERENCES users(id)
) STRICT;

CREATE TABLE message_attachments (
    message_version_id TEXT,
    media_id TEXT,
    FOREIGN KEY (media_id) REFERENCES media(id),
    FOREIGN KEY (message_version_id) REFERENCES messages(version_id)
) STRICT;

-- TODO: test
CREATE TRIGGER cleanup_message_media AFTER DELETE ON messages BEGIN
    UPDATE media SET deleted_at = datetime() WHERE id IN 
        (SELECT id FROM message_attachments WHERE message_version_id = OLD.version_id);
END;

-- CREATE TABLE user_relations (
--     user_id TEXT PRIMARY KEY,
--     other_id TEXT NOT NULL,
--     note TEXT NOT NULL,
--     status TEXT NOT NULL,
--     FOREIGN KEY (user_id) REFERENCES user(user_id),
--     FOREIGN KEY (other_id) REFERENCES user(other_id),
-- );

-- CREATE TABLE inbox (
--     item_id INTEGER PRIMARY KEY AUTOINCREMENT,
--     user_id TEXT NOT NULL,
--     data TEXT NOT NULL,
--     FOREIGN KEY (user_id) REFERENCES users(user_id)
-- ) STRICT;

CREATE TABLE config (key TEXT PRIMARY KEY, value ANY);

CREATE VIEW messages_coalesced AS
    SELECT *
    FROM (SELECT *, ROW_NUMBER() OVER(PARTITION BY id ORDER BY version_id DESC) AS row_num
        FROM messages)
    WHERE row_num = 1;

CREATE VIEW messages_counts AS
    SELECT thread_id, count(*)
    FROM message_coalesced
    GROUP BY thread_id;

CREATE TABLE invites (
    target_type TEXT NOT NULL,
    target_id TEXT NOT NULL,
    code TEXT NOT NULL,
    creator_id TEXT NOT NULL,
    FOREIGN KEY (creator_id) REFERENCES users(id)
) STRICT;

INSERT INTO users (id, parent_id, name, description, status, is_bot, is_alias, is_system, can_fork)
VALUES ('01940be4-6ca5-7351-ac71-914f49f9824c', null, 'tezlm', null, null, 0, 0, 0, 1);
INSERT INTO sessions (id, token, user_id, status) VALUES ('01940be4-b547-7b8e-b3f0-63900545a0f9', 'abcdefg', '01940be4-6ca5-7351-ac71-914f49f9824c', 1);
-- UPDATE sessions SET status = 1 WHERE status = 2;
