alter table script_log add column source jsonb not null default '{}';

update script_log set source = jsonb_build_object(
    'type', 'Script',
    'target', target,
    'span_start', span_start,
    'span_end', span_end
);

alter table script_log
    drop column target,
    drop column span_start,
    drop column span_end;
