CREATE TABLE calendar_overwrite (
    event_id UUID NOT NULL REFERENCES calendar_event(id) ON DELETE CASCADE,
    seq BIGINT NOT NULL,
    title TEXT,
    description TEXT,
    location TEXT,
    url TEXT,
    start_at TIMESTAMP,
    end_at TIMESTAMP,
    cancelled BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (event_id, seq)
);

CREATE TABLE calendar_overwrite_rsvp (
    event_id UUID NOT NULL,
    seq BIGINT NOT NULL,
    user_id UUID NOT NULL REFERENCES usr(id) ON DELETE CASCADE,
    attending BOOLEAN NOT NULL,
    PRIMARY KEY (event_id, seq, user_id),
    FOREIGN KEY (event_id, seq) REFERENCES calendar_overwrite(event_id, seq) ON DELETE CASCADE
);
