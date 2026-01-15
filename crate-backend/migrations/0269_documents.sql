create type branch_state as enum ('Active', 'Closed', 'Merged');

create table document_branch (
    id uuid primary key,
    document_id uuid not null references channel(id) on delete cascade,
    creator_id uuid not null references usr(id),
    name text,
    created_at timestamp not null default now(),
    is_default boolean not null default false,
    private boolean not null default false,
    state branch_state not null default 'Active',
    parent_branch_id uuid references document_branch(id)
);

create table document_snapshot (
    id uuid primary key,
    document_id uuid not null references channel(id) on delete cascade,
    branch_id uuid not null references document_branch(id) on delete cascade,
    snapshot bytea not null,
    seq int not null,
    created_at timestamp not null default now()
);

create table document_update (
    document_id uuid not null references channel(id) on delete cascade,
    branch_id uuid not null references document_branch(id) on delete cascade,
    snapshot_id uuid not null references document_snapshot(id) on delete cascade,
    seq int not null,
    data bytea not null,
    author_id uuid not null, -- intentionally no foreign key here!
    created_at timestamp not null default now(),
    primary key (branch_id, seq)
);