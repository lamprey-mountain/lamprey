# media

lamprey's media API handles file uploads, downloads, and processing (thumbnails,
transcoding).

NOTE: kinda tus compatible?

## uploading

Lamprey supports a multi-step upload process for large files and a direct upload
for small ones.

### resumable upload (recommended)

1. `POST /api/v1/media` – create a new media item. returns a `media_id` and an
   `upload_url`.
   - set `source` to `Upload` with `filename` and optional `size`.
2. `PATCH {upload_url}` – upload a chunk of the file.
   - headers: `Upload-Offset`, `Content-Length`.
3. `PUT /api/v1/media/{media_id}/done` – finishes the upload and begins
   processing.
   - body: `{"process_async": true}`.

### direct upload

- `POST /api/v1/media/direct` – upload a file directly using
  `multipart/form-data`.
  - fields: `file`.

### imports

- `POST /api/v1/media` – set `source` to `Download` with `source_url` to have
  the server import media from an external URL.

## retrieving

CDN routes (served by `crate-media`):

- `GET /media/{media_id}` – get original file.
- `GET /media/{media_id}/{original_filename}` – get original file with filename.
- `GET /thumb/{media_id}?size=[64|320|640]` – get thumbnail.
- `GET /emoji/{emoji_id}?size=[64|320|640]` – get thumbnail by custom emoji id.
- `GET /gifv/{media_id}` – get transcoded video for a GIF.

## other operations

- `GET /api/v1/media/{media_id}` – get media metadata.
- `PATCH /api/v1/media/{media_id}` – update properties (e.g. `alt` text).
- `DELETE /api/v1/media/{media_id}` – delete media (only if not linked to any
  resource).
