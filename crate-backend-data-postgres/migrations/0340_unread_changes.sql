alter table unread drop column version_id;
alter table unread add column pins_read_at timestamp;
