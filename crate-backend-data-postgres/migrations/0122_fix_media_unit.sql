update media
set data = jsonb_set(
  data,
  '{tracks}',
  (
    select jsonb_agg(
      jsonb_set(track - 'size_type', '{size_unit}', track -> 'size_type')
    )
    from jsonb_array_elements(data->'tracks') as track
  )
)
where data->'tracks' is not null;
