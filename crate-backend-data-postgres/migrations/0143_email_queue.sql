create table email_queue (
    id uuid primary key,
    to_addr text not null,
    from_addr text not null,
    subject text not null,
    body text not null,
    status text not null default 'pending',
    retries integer not null default 0,
    last_attempt_at timestamp,
    error_message text,
    created_at timestamp not null default now(),
    claimed_at timestamp,
    finished_at timestamp
);
