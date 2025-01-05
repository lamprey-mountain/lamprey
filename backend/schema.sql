CREATE INDEX messages_message_id ON messages (id);

CREATE UNIQUE INDEX users_email ON users (email);

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

INSERT INTO users (id, parent_id, name, description, status, is_bot, is_alias, is_system, can_fork)
VALUES ('01940be4-6ca5-7351-ac71-914f49f9824c', null, 'tezlm', null, null, false, false, false, true);
INSERT INTO sessions (id, token, user_id, status) VALUES ('01940be4-b547-7b8e-b3f0-63900545a0f9', 'abcdefg', '01940be4-6ca5-7351-ac71-914f49f9824c', 1);
-- UPDATE sessions SET status = 1 WHERE status = 2;
