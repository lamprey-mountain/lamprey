create table oauth_refresh_token (
    token text primary key,
    session_id uuid not null references session(id) on delete cascade,
    created_at timestamp not null
);
