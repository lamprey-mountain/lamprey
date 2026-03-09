update usr set bot_is_bridge = coalesce(bot_is_bridge, false);
alter table usr alter column bot_is_bridge set default false;
alter table usr alter column bot_is_bridge set not null;
