ALTER TYPE channel_type ADD VALUE 'Calendar';

CREATE TABLE calendar_event (
    id UUID PRIMARY KEY,
    channel_id UUID NOT NULL REFERENCES channel(id) ON DELETE CASCADE,
    creator_id UUID REFERENCES usr(id),
    title TEXT NOT NULL,
    description TEXT,
    location TEXT,
    url TEXT,
    timezone TEXT,
    recurrence JSONB,
    start_at TIMESTAMP NOT NULL,
    end_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMP
);

CREATE TABLE calendar_event_rsvp (
    event_id UUID NOT NULL REFERENCES calendar_event(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES usr(id) ON DELETE CASCADE,
    PRIMARY KEY (event_id, user_id)
);
