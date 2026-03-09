-- https://stackoverflow.com/questions/75869246/how-to-extract-timestamp-from-uuid-v7
CREATE OR REPLACE FUNCTION extract_timestamp_from_uuid_v7(uuid_v7 UUID)
RETURNS TIMESTAMP AS $$
  SELECT to_timestamp(('x'||replace(uuid_v7::text, '-', ''))::bit(48)::bigint / 1000) AS result;
$$ LANGUAGE sql IMMUTABLE;
