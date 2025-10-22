# media

this is media cdn proxy that can generate thumbnails

## notes

http routes

- `GET /media/{media_id}`
- `GET /media/{media_id}/{original_filename}`
- `GET /thumb/{media_id}?size=[64|320|640]`
- `GET /emoji/{emoji_id}?size=[64|320|640]`

s3 paths

- `s3://chat-files/media/{media_id}`
  - `/file` the original uploaded file
  - `/poster` the extracted thumbnail for video/audio files
  - `/thumb/{size}x{size}.{ext}` generated thumbnails (eg. `/thumb/64x64.avif`)
