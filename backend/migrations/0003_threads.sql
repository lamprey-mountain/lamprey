CREATE TABLE IF NOT EXISTS thread (
    type INT NOT NULL,
    id UUID PRIMARY KEY,
    room_id UUID NOT NULL,
    creator_id UUID NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    is_closed BOOL NOT NULL,
    is_locked BOOL NOT NULL,
    deleted_at INT,
    FOREIGN KEY (room_id) REFERENCES room(id),
    FOREIGN KEY (creator_id) REFERENCES usr(id)
);
