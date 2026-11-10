# media

lamprey stores media metadata in the database while the actual bytes live in
blob storage (an s3-compatible bucket or the local filesystem). management
routes live under `/api/v1/media`; uploads and downloads use separate routes,
usually on a different host (`cdn_url`).

creating, updating, and deleting media requires an authenticated session with
the `Full` scope. `GET /api/v1/media/{media_id}` is public.

## lifecycle

`MediaStatus` describes where a piece of media is in its lifecycle:

- `Transferring`: intended for an upload or download that is still in
  progress. the current rest path inserts `Processing` immediately instead.
- `Processing`: bytes are being transferred or inspected.
- `Uploaded`: processing finished and the blob can be fetched from the cdn.
- `Consumed`: the media has at least one active link. the blob is immutable and
  `strip_exif` can no longer be changed. this status is derived when the media
  is read, not stored separately.
- `Errored`: processing failed.

an upload that is not touched for 5 minutes is dropped.

processing sniffs the mime type, extracts metadata with ffprobe, calculates
blake3 and sha512/256 hashes, optionally strips exif, runs configured media
scanners, extracts a poster for audio/video, and uploads the blob. thumbnails
and gifv transcodes are generated lazily by the cdn routes, not during upload.

## uploading

uploads are resumable but are not tus-compatible.

1. `POST /api/v1/media` creates a media record. the body is a `MediaCreate`:

   ```json
   {
     "alt": "a cat",
     "strip_exif": true,
     "filename": "cat.png",
     "size": 123456
   }
   ```

   `filename` and the optional `size` identify an upload. `size` is strongly
   recommended because it lets the server reject oversized files early.
   `strip_exif` is applied during processing and cannot be turned back off.

   the response is `201 Created` with a `MediaCreated` body and `upload-offset`
   / `upload-length` headers:

   ```json
   { "media_id": "...", "upload_url": "https://.../api/v1/internal/media-upload/..." }
   ```

2. `PATCH {upload_url}` uploads a chunk. send `Upload-Offset` and
   `Content-Length` headers; the chunk is appended at that offset. the final
   chunk automatically starts processing in the background.

   `HEAD {upload_url}` returns the current `upload-offset` and `upload-length`,
   which is useful for resuming. only the uploader can read those headers.

   the upload route authenticates the session but does not currently verify
   that the caller owns the media being uploaded.

3. `PUT /api/v1/media/{media_id}/done` waits for processing. the body is
   `{ "async": true }` or `{ "async": false }`.

   - `async: false` (default) waits and returns `200` with the finished media.
   - `async: true` returns `202` immediately with a `null` media.

   a `MediaProcessed` sync event is sent to any sync connections open for the
   uploader session id when the upload is done processing.

### importing from a url

send `source_url` instead of `filename`. `filename` and `size` are optional
when importing, but a size is still recommended.

```json
{ "source_url": "https://example.com/cat.png", "size": 123456 }
```

the server downloads the url, checks the declared/actual size, and processes
the file before responding. `MediaCreated.upload_url` is `null` for imports.

## downloading

cdn routes:

| route                                          | description                                                       |
| ---------------------------------------------- | ----------------------------------------------------------------- |
| `GET /media/{media_id}`                        | the original file                                                 |
| `GET /media/{media_id}/{filename}`             | the original file, but only if `filename` matches the stored name |
| `GET /thumb/{media_id}`                        | the original poster (no resizing)                                 |
| `GET /thumb/{media_id}?size=320`               | a generated thumbnail                                             |
| `GET /emoji/{emoji_id}?size=64`                | a custom emoji's thumbnail                                        |
| `GET /gifv/{media_id}`                         | a webm transcode of a gif                                         |
| `GET /harvest/{harvest_id}/{token}/{filename}` | a completed harvest archive (zip)                                 |

all of these support `HEAD`. file, thumb, and gifv routes support single byte
ranges, weak etags, and `If-None-Match` / `If-Modified-Since` / `If-Range`.

both the file and thumbnail routes accept `?wait=true|false`. the option is
defined but currently ignored: these routes always wait for processing to
finish. media that has not finished processing will block the request.

thumbnail sizes are configured by the server. if the requested size does not
exist, the server picks the smallest configured size that is at least as large,
falling back to the largest configured size. `?animate=false` uses a static
avif thumbnail; `?animate=true` (the default) allows an animated webp
thumbnail.

the gifv route only works for `image/gif`. the stored blob is webm; the current
response advertises it as `video/mp4`.

harvest routes are public but require the exact id, token, and filename embedded
in the download url. the link expires with the harvest.

## managing media

- `GET /api/v1/media/{media_id}` – get metadata. accepts a local id or a
  federated id. deleted media is only visible to server admins.
- `PATCH /api/v1/media/{media_id}` – update `alt`, `filename`, or `strip_exif`.
  only the uploader can patch their media. `strip_exif` cannot be changed on
  consumed media and cannot be changed back to `false`. setting it to `true`
  updates the flag but does not rewrite the already-uploaded blob.
- `DELETE /api/v1/media/{media_id}` – delete unlinked media. linked media
  returns `409 Conflict`.
- `POST /api/v1/media/{media_id}/quarantine` – quarantine media (server admin
  only). broadcasts `MediaUpdate`.
- `DELETE /api/v1/media/{media_id}/quarantine` – remove a quarantine (server
  admin only).
- `POST /api/v1/media/search` – search media (server admin only).
- `POST /api/v1/media/{media_id}/clone` – defined but currently returns
  `Unimplemented`.

`Media` includes `user_id`, `deleted_at`, `quarantine`, `scans`, `hashes`,
`links`, `has_thumbnail`, `has_gifv`, and `remote` for federated media.
`content_type` is a `Mime` and `metadata` is a `MediaMetadata` variant:
`Image`, `Video`, `Audio`, `Text`, `File`, or `Errored`.

the type documents `user_id`, `deleted_at`, and `quarantine` as
uploader/admin-only fields, but the current media routes return the stored
`Media` directly without trimming them.

## s3 paths

`MediaPaths` computes blob keys. with the default `media/` prefix:

- `media/{media_id}/file` – the original uploaded file
- `media/{media_id}/poster` – extracted poster for audio/video
- `media/{media_id}/thumb/{size}x{size}.webp` – animated thumbnail
- `media/{media_id}/thumb/{size}x{size}_static.avif` – static thumbnail
- `media/{media_id}/gifv` – webm gif transcode

the cdn does not expose trickplay or segmented stream routes yet; their route
types exist in `crate-common`, but they are not registered on the media server.
