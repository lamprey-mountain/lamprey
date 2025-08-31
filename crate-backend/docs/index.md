You've found the API docs!

## Resources

* https://git.celery.eu.org/lamprey/lamprey – source code
* https://github.com/tezlm/lamprey – readonly mirror
* https://chat.celery.eu.org – current "production" instance

## reference

- versioning: v1 is the only version, and its unstable
- authentication: Bearer token in authorization header
- dates/times use [rfc3339](https://datatracker.ietf.org/doc/html/rfc3339)

cdn routes:

- `GET /media/{media_id}`
- `GET /media/{media_id}/{original_filename}`
- `GET /thumb/{media_id}?size=[64|320|640]`
- `GET /emoji/{emoji_id}?size=[64|320|640]`

## syncing

- TODO: write more/better docs about syncing here
- TODO: deduplicate with sync.md? or copy everything here

how to sync:

1. Open a WebSocket connection to `wss://chat.celery.eu.org/api/v1/sync`
2. Send a `Hello` message
3. Receive a `Ready` message, containing user, session, and connection id
4. Every time you receive a `Ping`, immediately respond with a `Pong`
5. Updates are sent via `Sync`, remember the last `seq` you saw
6. If you get a `Reconnect`, disconnect and start a new connection. Resume if
   `can_resume` is true.
7. `Error`s aren't necessarily fatal, you'll get a `Reconnect` if you need to
   reconnect.

To resume, send the same `conn` from the initial `Ready` event and last `seq`
number you saw. You will receive a `Resume` event once you have all missing
events.

Query params `version=1`, `format=json` (only supported format currently),
`compress=none` (compression will be added later)

## permissions

- TODO: explain how permissions are resolved

## media

1. `POST /api/v1/media` – create a new media/upload item. get
2. `PATCH /api/v1/media/{media_id}` – upload. use the headers specified below when uploading
3. ignore media_done for now, currently backend automatically starts processing once a file is fully received

headers:
- `Upload-Offset` – resuming uploads
- `Upload-Length` – the total size of the file
- `Content-Length` – the size of this chunk

## voice

- TODO: explain how to use webrtc to connect to the voice thing
