truncate audit_log;
alter table audit_log drop column payload_prev;
alter table audit_log drop column payload;
alter table audit_log add column session_id uuid;
alter table audit_log add column data jsonb;
