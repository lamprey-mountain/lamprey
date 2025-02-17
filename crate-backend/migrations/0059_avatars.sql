alter table usr drop column avatar_url;
alter table usr add column avatar uuid references media (id);
alter type media_link_type add value 'AvatarUser';
