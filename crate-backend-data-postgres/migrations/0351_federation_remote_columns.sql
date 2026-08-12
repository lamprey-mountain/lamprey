alter table room add column remote_origin_id uuid, add column remote_hostname text;
create index room_remote_lookup on room (remote_hostname, remote_origin_id) where remote_origin_id is not null;

alter table channel add column remote_origin_id uuid, add column remote_hostname text;
create index channel_remote_lookup on channel (remote_hostname, remote_origin_id) where remote_origin_id is not null;

alter table invite add column remote_origin_id uuid, add column remote_hostname text;
create index invite_remote_lookup on invite (remote_hostname, remote_origin_id) where remote_origin_id is not null;
