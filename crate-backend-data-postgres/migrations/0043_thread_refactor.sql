create type thread_state as enum ('Pinned', 'Active', 'Temporary', 'Archived', 'Deleted');
create type thread_visibility as enum ('Room');
alter table thread add column state thread_state not null default 'Active';
alter table thread add column visibility thread_visibility not null default 'Room';
alter table thread alter column state drop default;
alter table thread alter column visibility drop default;
alter type thread_type rename value 'Default' to 'Chat';
