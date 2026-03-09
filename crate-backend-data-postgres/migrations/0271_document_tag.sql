create table document_tag (
    id uuid primary key,
    branch_id uuid not null references document_branch(id) on delete cascade,
    revision_seq bigint not null,
    creator_id uuid references usr(id) on delete set null,
    created_at timestamp not null default now(),
    updated_at timestamp not null default now(),
    summary text not null,
    description text
);
