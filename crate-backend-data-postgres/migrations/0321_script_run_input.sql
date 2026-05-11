alter table script_run add column input jsonb not null default '{"type":"Extraction"}';
