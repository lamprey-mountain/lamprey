CREATE TABLE IF NOT EXISTS room_members (
    room_id UUID,
    user_id UUID,
    membership TEXT NOT NULL,
    override_name TEXT,
    override_description TEXT,
    FOREIGN KEY (room_id) REFERENCES rooms(id),
    FOREIGN KEY (user_id) REFERENCES users(id),
    PRIMARY KEY (room_id, user_id)
);
