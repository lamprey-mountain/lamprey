alter table inbox drop column reason;
drop type if exists notification_reason;
create type notification_type as enum ('Message', 'Reaction');
alter table inbox add column type notification_type not null default 'Message';
