alter table invite rename column remote_origin_id to remote_origin_code;
alter table invite alter column remote_origin_code type text;
