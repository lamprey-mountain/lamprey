alter table url_embed_queue add column message_ref jsonb;
alter table url_embed_queue drop column message_version_id;
alter table url_embed_queue drop column thread_id;
