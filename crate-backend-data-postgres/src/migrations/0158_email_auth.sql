create table email_auth_code (
    code text primary key,
    addr text not null,
    session_id uuid not null,
    created_at timestamp not null default now(),
    expires_at timestamp not null,
    purpose text not null,
    foreign key (session_id) references session(id) on delete cascade
);
