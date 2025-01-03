CREATE TABLE IF NOT EXISTS room_members (
    id TEXT PRIMARY KEY,
    user_id TEXT,
    room_id TEXT,
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (room_id) REFERENCES rooms(id)
);
