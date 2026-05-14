alter table redex_eval add column redex_version_id uuid not null default '00000000-0000-0000-0000-000000000000';

update redex_eval
set redex_version_id = (select version_id from redex_version where script_id = redex_eval.script_id limit 1)
where redex_version_id = '00000000-0000-0000-0000-000000000000';

alter table redex_eval alter column redex_version_id drop default;

alter table redex_eval add constraint redex_eval_redex_version_id_fkey
    foreign key (redex_version_id) references redex_version(version_id) on delete cascade;
