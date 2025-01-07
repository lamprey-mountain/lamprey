CREATE TABLE IF NOT EXISTS room_member (
    room_id UUID,
    user_id UUID,
    membership TEXT NOT NULL,
    override_name TEXT,
    override_description TEXT,
    FOREIGN KEY (room_id) REFERENCES room(id),
    FOREIGN KEY (user_id) REFERENCES usr(id),
    PRIMARY KEY (room_id, user_id)
);
