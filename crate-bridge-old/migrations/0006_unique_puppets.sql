delete from puppet;
create unique index puppet_ext_key on puppet(ext_platform, ext_id);
