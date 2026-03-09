alter type bot_visibility_type rename to bot_access_type;
alter table usr rename column bot_visibility to bot_access;
