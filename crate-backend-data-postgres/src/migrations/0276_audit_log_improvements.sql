create type audit_log_entry_status as enum ('Success', 'Unauthorized', 'Failed');

alter table audit_log add column status audit_log_entry_status not null default 'Success';
alter table audit_log alter column status drop default;

alter table audit_log add column ended_at timestamp;
update audit_log set ended_at = extract_timestamp_from_uuid_v7(id);
alter table audit_log alter column ended_at set not null;

alter table audit_log add column started_at timestamp;
update audit_log set started_at = extract_timestamp_from_uuid_v7(id);
alter table audit_log alter column started_at set not null;

alter table audit_log add column ip_addr inet;
alter table audit_log add column user_agent text;
alter table audit_log add column application_id uuid references application(id);

create index on audit_log (status);
