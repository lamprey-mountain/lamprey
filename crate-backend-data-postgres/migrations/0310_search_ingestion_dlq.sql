create table if not exists search_ingestion_dlq (
    id uuid primary key,
    entity_id uuid not null,
    entity_type text not null,
    error_message text not null,
    created_at timestamp not null default now()
);
