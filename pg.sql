CREATE TABLE rooms (
    room_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT
);

CREATE TABLE role (
    role_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    permissions INT NOT NULL
);

CREATE TABLE role_application (
    user_id INT,
    role_id INT,
    FOREIGN KEY (user_id) REFERENCES users(user_id),
    FOREIGN KEY (role_id) REFERENCES roles(role_id),
    PRIMARY KEY (user_id, role_id)
);

CREATE TABLE threads (
    thread_id TEXT PRIMARY KEY,
    room_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    is_closed INT NOT NULL,
    is_locked INT NOT NULL,
    FOREIGN KEY (room_id) REFERENCES rooms(room_id)
);

CREATE TABLE messages (
    message_id TEXT NOT NULL,
    thread_id TEXT NOT NULL,
    version_id TEXT PRIMARY KEY,
    ordering INT NOT NULL,
    content TEXT NOT NULL,
    metadata TEXT NOT NULL,
    reply TEXT,
    author_id TEXT NOT NULL,
    FOREIGN KEY (thread_id) REFERENCES threads(thread_id),
    FOREIGN KEY (author_id) REFERENCES users(user_id)
    -- FOREIGN KEY (reply) REFERENCES messages(message_id)
);

CREATE INDEX messages_message_id ON messages (message_id);

CREATE TABLE users (
    user_id TEXT PRIMARY KEY,
    name TEXT,
    description TEXT,
    avatar_url TEXT,
    is_bot INT
);

CREATE TABLE sessions (
    session_id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    token TEXT NOT NULL,
    name TEXT,
    FOREIGN KEY (user_id) REFERENCES users(user_id)
);

CREATE TABLE room_members (
    member_id TEXT PRIMARY KEY,
    user_id TEXT,
    room_id TEXT,
    FOREIGN KEY (user_id) REFERENCES users(user_id),
    FOREIGN KEY (room_id) REFERENCES rooms(room_id)
);

CREATE TABLE thread_members (
    member_id TEXT NOT NULL,
    thread_id TEXT NOT NULL,
    FOREIGN KEY (member_id) REFERENCES members(member_id),
    FOREIGN KEY (thread_id) REFERENCES threads(thread_id),
    PRIMARY KEY (member_id, thread_id)
);

CREATE TABLE media (
    media_id TEXT PRIMARY KEY,
    purpose TEXT NOT NULL,
    size INT NOT NULL,
    uploader TEXT NOT NULL,
    deleted_at TEXT,
    FOREIGN KEY (uploader) REFERENCES users(user_id)
);

CREATE TABLE message_attachments (
    message_version_id TEXT,
    media_id TEXT,
    FOREIGN KEY (media_id) REFERENCES media(media_id),
    FOREIGN KEY (message_version_id) REFERENCES messages(version_id)
);

-- TODO: test
-- CREATE TRIGGER cleanup_message_media AFTER DELETE ON messages BEGIN
--     UPDATE media SET deleted_at = datetime() WHERE media_id IN 
--         (SELECT media_id FROM message_attachments WHERE message_version_id = OLD.version_id);
-- END;

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
-- );

CREATE TABLE config (key TEXT PRIMARY KEY, value BLOB);

CREATE VIEW messages_coalesced AS
    SELECT *
    FROM (SELECT *, ROW_NUMBER() OVER(PARTITION BY message_id ORDER BY version_id DESC) AS row_num
        FROM messages)
    WHERE row_num = 1;

-- CREATE VIEW message_counts AS
--   SELECT thread_id, count(*)
--   FROM (SELECT *, ROW_NUMBER() OVER(PARTITION BY message_id ORDER BY version_id DESC) AS row_num
--         FROM messages)
--   WHERE row_num = 1 GROUP BY thread_id;

CREATE VIEW messages_counts AS
    SELECT thread_id, count(*)
    FROM message_coalesced
    GROUP BY thread_id;

-- INSERT INTO users (user_id, name) VALUES ('01940be4-6ca5-7351-ac71-914f49f9824c', 'tezlm');
-- INSERT INTO sessions (session_id, token, user_id) VALUES ('01940be4-b547-7b8e-b3f0-63900545a0f9', 'abcdefg', '01940be4-6ca5-7351-ac71-914f49f9824c');

