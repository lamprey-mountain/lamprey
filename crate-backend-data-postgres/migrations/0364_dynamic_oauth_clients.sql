alter table application
    add column created_at timestamp not null default now(),
    add column oauth_last_token_issued_at timestamp;

-- dynamically registered clients are owned by DYNAMIC_APP_OWNER_ID
create index application_owner_id_created_at_idx
    on application (owner_id, created_at);
