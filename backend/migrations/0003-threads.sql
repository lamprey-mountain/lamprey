CREATE TABLE IF NOT EXISTS threads (
    id UUID PRIMARY KEY,
    room_id UUID NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    is_closed BOOL NOT NULL,
    is_locked BOOL NOT NULL,
    FOREIGN KEY (room_id) REFERENCES rooms(id)
);
