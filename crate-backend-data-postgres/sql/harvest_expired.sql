SELECT data FROM harvest
WHERE (data->>'status' = 'Completed' AND (data->>'expires_at')::timestamptz <= $1)
   OR (data->>'status' IN ('Failed', 'Cancelled') AND (queued_at at time zone 'UTC') < $2)
