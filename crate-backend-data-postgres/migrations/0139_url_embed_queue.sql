create table url_embed_queue (
    id uuid primary key,
    message_id uuid,
    thread_id uuid,
    user_id uuid not null,
    url text not null,
    created_at timestamp not null default now(),
    claimed_at timestamp,
    finished_at timestamp,
    foreign key (thread_id) references thread(id) on delete cascade
);

create index url_embed_queue_pending_idx on url_embed_queue (claimed_at, created_at) where finished_at is null;
