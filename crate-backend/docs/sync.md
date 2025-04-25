todo: write more/better docs about syncing here

# how to sync

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
