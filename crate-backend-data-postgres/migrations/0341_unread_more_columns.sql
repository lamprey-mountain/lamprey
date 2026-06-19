-- these aren't used yet, but i might as well add them now
alter table unread add column unread_count int not null default 0;
alter table unread add column last_viewed timestamp;
