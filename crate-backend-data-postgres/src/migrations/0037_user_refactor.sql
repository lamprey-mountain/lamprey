create type user_type as enum ('Default', 'Alias', 'Bot', 'System');
create type user_state as enum ('Active', 'Suspended', 'Deleted');

alter table usr add column type user_type;

update usr set type = (
    case
        when is_bot then 'Bot'
        when is_alias then 'Alias'
        when is_system then 'System'
        else 'Default'
    end :: user_type
);

alter table usr drop column is_bot;
alter table usr drop column is_alias;
alter table usr drop column is_system;
alter table usr drop column deleted_at;
alter table usr add column state_updated_at timestamp default now();
alter table usr add column state user_state default 'Active' not null;
alter table usr alter column state drop default;
