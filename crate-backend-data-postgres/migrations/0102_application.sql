create table application (
    id uuid primary key,
    owner_id uuid not null,
    name text not null,
    description text,
    bridge boolean not null,
    public boolean not null
);
