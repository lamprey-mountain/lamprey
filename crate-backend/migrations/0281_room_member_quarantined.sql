alter table room_member add column quarantined boolean not null default false;
alter table room_member alter column quarantined drop default;
