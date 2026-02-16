alter table thread add column user_limit int;
alter table thread add column bitrate int;
update thread set bitrate = 64000 where type = 'Voice';
