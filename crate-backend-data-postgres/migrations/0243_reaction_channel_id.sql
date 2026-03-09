alter table reaction add column channel_id uuid references channel (id) on delete cascade;
