alter table role rename permissions to allow;
alter table role add column deny permission[] NOT NULL default '{}';
alter table role alter column deny drop default;
