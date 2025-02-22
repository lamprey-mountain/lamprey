alter table url_embed add column user_id uuid not null references usr (id);
