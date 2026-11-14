alter table channel add column pinned_thread_ids uuid[] not null default '{}';
