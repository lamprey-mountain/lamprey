alter table room_member add column mute boolean not null default false;
alter table room_member add column deaf boolean not null default false;
alter table room_member alter column mute drop default;
alter table room_member alter column deaf drop default;
