CREATE TABLE IF NOT EXISTS invites (
    target_type TEXT NOT NULL,
    target_id UUID NOT NULL,
    code TEXT NOT NULL,
    creator_id UUID NOT NULL,
    FOREIGN KEY (creator_id) REFERENCES users(id)
);
