alter table room add column welcome_thread_id uuid references thread (id);
