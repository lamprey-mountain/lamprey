alter table usr add column user_version uuid;
update usr set user_version = id;
alter table usr alter column user_version set not null;
