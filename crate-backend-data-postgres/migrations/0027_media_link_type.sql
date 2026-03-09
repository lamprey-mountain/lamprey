create type media_link_type as enum ('Message', 'MessageVersion');
alter table media_link alter column link_type type media_link_type using case link_type when 0 then 'Message'::media_link_type when 1 then 'MessageVersion'::media_link_type end;
