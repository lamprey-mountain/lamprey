alter table unread add column message_id uuid;
update unread set message_id = (select id from message where message.version_id = unread.version_id);
alter table unread alter column message_id set not null;
