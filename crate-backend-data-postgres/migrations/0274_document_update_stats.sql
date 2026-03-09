alter table document_update add column stat_added int not null default 0;
alter table document_update add column stat_removed int not null default 0;
alter table document_update alter column stat_added drop not null;
alter table document_update alter column stat_removed drop not null;
