create table permission_overwrite (
    target_id uuid not null,
    actor_id uuid not null,
    allow permission[] not null,
    deny permission[] not null,
    primary key (target_id, actor_id)
);
