# media

lamprey's media API handles file uploads, downloads, and processing (thumbnails,
transcoding).

<!-- NOTE: the api is kinda tus compatible but not really right now. maybe i should fully lean into tus? -->

## uploading

For most files, you'll want to use the resumeable upload api.

1. `POST /api/v1/media` – create a new media item. accepts `filename` and
   `size`. returns a `media_id` and an `upload_url`.
2. `PATCH {upload_url}` – upload a chunk of the file.
   set `Upload-Offset` and `Content-Length` headers.
3. `PUT /api/v1/media/{media_id}/done` – finishes the upload and begins
   processing. you can use `{"process_async": true}` to process media in the
   background, you'll receive a `MediaProcessed` event once your media is done
   processing.

You can also import media from a url by using the create media endpoint with a
`source_url` field.

## downloading

CDN routes:

- `GET /media/{media_id}` – get original file.
- `GET /media/{media_id}/{original_filename}` – get original file with filename.
- `GET /thumb/{media_id}?size=[64|320|640]` – get thumbnail.
- `GET /emoji/{emoji_id}?size=[64|320|640]` – get thumbnail by custom emoji id.
- `GET /gifv/{media_id}` – get transcoded video for a GIF.

Range requests are supported. All media also has etag/last-modified headers set. Media is immutable for each path.

## other operations

- `GET /api/v1/media/{media_id}` – get media metadata.
- `PATCH /api/v1/media/{media_id}` – update metadata (e.g. `alt` text).
- `DELETE /api/v1/media/{media_id}` – delete media (only if not linked to any
  resource).

## s3 paths

- `s3://chat-files/media/{media_id}`
  - `/file` the original uploaded file
  - `/poster` the extracted thumbnail for video/audio files
  - `/thumb/{size}x{size}.{ext}` generated thumbnails (eg. `/thumb/64x64.avif`)
  - `/thumb/{size}x{size}_static.{ext}` static generated thumbnails, with animation removed (eg. `/thumb/64x64_static.avif`)
  - `/gifv` if this is a gif, this is its webm transcoding
- see kerosene-core/src/types/media.rs for more information
