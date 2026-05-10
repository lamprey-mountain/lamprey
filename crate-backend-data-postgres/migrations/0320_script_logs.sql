create table script_run (
    id uuid primary key,
    script_id uuid not null references script(id) on delete cascade,
    created_at timestamp not null,
    stopped_at timestamp,
    status smallint not null
);

create table script_log (
    run_id uuid not null references script_run(id) on delete cascade,
    line_id bigint not null,
    created_at timestamp not null,
    level smallint not null,
    target text not null,
    span_start bigint not null,
    span_end bigint not null,
    content text not null,
    attributes jsonb not null default '{}',
    primary key (run_id, line_id)
);

create index idx_script_run_script_id on script_run(script_id);
