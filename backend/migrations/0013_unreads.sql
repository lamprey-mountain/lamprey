CREATE TABLE IF NOT EXISTS unread (
    thread_id UUID NOT NULL,
    user_id UUID NOT NULL,
    version_id UUID NOT NULL,
    -- is_unread BOOL NOT NULL,
    PRIMARY KEY (thread_id, user_id),
    FOREIGN KEY (thread_id) REFERENCES thread (id),
    FOREIGN KEY (user_id) REFERENCES usr (id),
    FOREIGN KEY (version_id) REFERENCES message (version_id)
);
