create temp view to_delete as (
    select id, version_id from message
    where deleted_at is not null
      and deleted_at < now() - interval '7 day'
);

update media set deleted_at = now() where id in (
    select media_id from media_link
    where (target_id in (select version_id from to_delete) and link_type = 'MessageVersion')
       or (target_id in (select id from to_delete) and link_type = 'Message')
);

delete from message_attachment where version_id in (select version_id from to_delete);
delete from media_link where target_id in (select id from to_delete) and link_type = 'Message';
delete from media_link where target_id in (select version_id from to_delete) and link_type = 'MessageVersion';
delete from media_link where target_id in (select id from to_delete) and link_type = 'Embed';
delete from message where version_id in (select version_id from to_delete);
delete from reaction where message_id in (select version_id from to_delete);

drop view to_delete;
