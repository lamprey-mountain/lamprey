-- Inbox notifications are not critical, so we can truncate them to add a primary key.
TRUNCATE inbox;
ALTER TABLE inbox ADD COLUMN id UUID PRIMARY KEY;
