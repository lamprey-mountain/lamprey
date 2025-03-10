alter table usr add column puppet_external_platform text;
alter table usr add column puppet_external_id text;
alter table usr add column puppet_external_url text;
alter table usr add column puppet_alias_id uuid references usr (id);
alter table usr add column bot_is_bridge boolean;
