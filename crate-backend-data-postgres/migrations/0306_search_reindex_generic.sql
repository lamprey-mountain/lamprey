alter table search_reindex_queue drop constraint if exists search_reindex_queue_channel_id_fkey;
alter table search_reindex_queue rename column channel_id to target_id;
alter table search_reindex_queue rename column last_message_id TO last_id;
alter table search_reindex_queue add column target_type text not null default 'channel';
alter table search_reindex_queue drop constraint if exists search_reindex_queue_pkey;
alter table search_reindex_queue add primary key (target_id, target_type);
