-- 	is_unread: z.boolean(),
-- 	last_read_id: MessageId,
-- 	last_message_id: MessageId,
-- 	message_count: z.number(),
-- 	mention_count: z.number(),

-- select threads.*, count as message_count
-- from threads
-- join messages_counts on thread_id = threads.id;

-- -- is_unread: z.boolean(),
-- -- last_read_id: MessageId,
-- -- last_message_id: MessageId,
-- -- message_count: z.number(),
-- -- mention_count: z.number(),
