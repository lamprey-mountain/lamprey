CREATE TYPE automod_target AS ENUM ('Content', 'Member');

ALTER TABLE automod_rule ADD COLUMN except_nsfw BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE automod_rule ADD COLUMN include_everyone BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE automod_rule ADD COLUMN target automod_target NOT NULL DEFAULT 'Content';

ALTER TABLE automod_rule ALTER COLUMN except_nsfw DROP DEFAULT;
ALTER TABLE automod_rule ALTER COLUMN include_everyone DROP DEFAULT;
ALTER TABLE automod_rule ALTER COLUMN target DROP DEFAULT;