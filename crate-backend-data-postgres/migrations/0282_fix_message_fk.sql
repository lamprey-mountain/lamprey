alter table message drop constraint fk_message_latest_version_id;
alter table message add constraint fk_message_latest_version_id foreign key (latest_version_id) references message_version(version_id) deferrable initially deferred;
