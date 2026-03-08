create table room_templates (
    code text primary key,
    name text not null,
    description text not null,
    created_at timestamp not null default now(),
    updated_at timestamp not null default now(),
    creator_id uuid not null references usr(id) on delete cascade,
    source_room_id uuid references room(id) on delete set null,
    snapshot jsonb not null,
    dirty boolean not null default false
);

create index room_templates_creator_id_idx on room_templates(creator_id);
