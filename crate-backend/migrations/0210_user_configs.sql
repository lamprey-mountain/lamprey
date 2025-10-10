CREATE TABLE user_config_room (
    user_id UUID NOT NULL REFERENCES usr(id) ON DELETE CASCADE,
    room_id UUID NOT NULL REFERENCES room(id) ON DELETE CASCADE,
    config JSONB NOT NULL,
    PRIMARY KEY (user_id, room_id)
);

CREATE TABLE user_config_thread (
    user_id UUID NOT NULL REFERENCES usr(id) ON DELETE CASCADE,
    thread_id UUID NOT NULL REFERENCES thread(id) ON DELETE CASCADE,
    config JSONB NOT NULL,
    PRIMARY KEY (user_id, thread_id)
);

CREATE TABLE user_config_user (
    user_id UUID NOT NULL REFERENCES usr(id) ON DELETE CASCADE,
    other_id UUID NOT NULL REFERENCES usr(id) ON DELETE CASCADE,
    config JSONB NOT NULL,
    PRIMARY KEY (user_id, other_id)
);
