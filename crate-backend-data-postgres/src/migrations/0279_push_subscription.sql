create table push_subscription (
    session_id uuid primary key references session(id) on delete cascade,
    user_id uuid not null references usr(id) on delete cascade,
    endpoint text not null,
    key_p256dh text not null,
    key_auth text not null,
    created_at timestamp not null default now()
);
