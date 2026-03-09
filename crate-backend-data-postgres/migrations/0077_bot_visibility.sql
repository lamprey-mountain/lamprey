create type bot_visibility_type as ENUM ('Private', 'Public', 'PublicDiscoverable');
alter table usr add column bot_visibility bot_visibility_type not null default 'Private';
