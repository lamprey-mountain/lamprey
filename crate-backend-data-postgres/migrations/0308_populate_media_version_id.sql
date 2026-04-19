update media
set
  version_id = id,
  data = jsonb_set(data, '{version_id}', to_jsonb(id::text))
where data->>'v' = 'V2' and data->>'version_id' is null;
