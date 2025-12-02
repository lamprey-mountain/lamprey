# media

kinda tus compatible, but not really since PATCH can return data currently

1. `POST /api/v1/media` – create a new media/upload item. get
2. `PATCH /api/v1/media/{media_id}` – upload. use the headers specified below
   when uploading
3. ignore media_done for now, currently backend automatically starts processing
   once a file is fully received

headers:

- `Upload-Offset` – resuming uploads
- `Upload-Length` – the total size of the file
- `Content-Length` – the size of this chunk

cdn routes:

- `GET /media/{media_id}`
- `GET /media/{media_id}/{original_filename}`
- `GET /thumb/{media_id}?size=[64|320|640]`
- `GET /emoji/{emoji_id}?size=[64|320|640]`
- `GET /gifv/{media_id}` (media must be a gif, will transcode to webm)
