alter table permission_overwrite alter column allow type jsonb using to_jsonb(allow);
alter table permission_overwrite alter column deny type jsonb using to_jsonb(deny);
