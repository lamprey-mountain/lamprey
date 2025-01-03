CREATE TABLE IF NOT EXISTS threads (
    id TEXT PRIMARY KEY,
    room_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    is_closed INT NOT NULL,
    is_locked INT NOT NULL,
    FOREIGN KEY (room_id) REFERENCES rooms(id)
);
