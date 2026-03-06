# sync

lamprey uses a websocket-based sync protocol to keep clients up to date with the
server state.

## how to sync

1. Open a WebSocket connection to `wss://chat.celery.eu.org/api/v1/sync`
2. Send a `Hello` message
3. Receive a `Ready` message, containing user, session, and connection id
4. Receive an `Ambient` message (via `Sync`), containing initial state (rooms,
   roles, channels, etc.)
5. Every time you receive a `Ping`, immediately respond with a `Pong`
6. Updates are sent via `Sync`, remember the last `seq` you saw
7. If you get a `Reconnect`, disconnect and start a new connection. Resume if
   `can_resume` is true.
8. `Error`s aren't necessarily fatal, you'll get a `Reconnect` if you need to
   reconnect.

To resume, send the same `conn` from the initial `Ready` event and last `seq`
number you saw in the `Hello` message's `resume` field. You will receive a
`Resumed` event once you have all missing events.

Query params:

- `version=1`
- `format=json` (only supported format currently)
- `compress=none` (compression will be added later)

## messages

### client to server (`MessageClient`)

- **Hello**: Initial message to authenticate and optionally resume.
- **Presence**: Update your current presence (e.g. online, away, busy).
- **Pong**: Response to a server `Ping`.
- **VoiceDispatch**: Send signalling data to a voice server.
- **MemberListSubscribe**: Subscribe to a range of room or thread members.
- **DocumentSubscribe**: Subscribe to a document (for real-time editing).
- **DocumentEdit**: Send an update to a document.
- **DocumentPresence**: Update your cursor position in a document.

### server to client (`MessagePayload`)

- **Ping**: Heartbeat from the server.
- **Sync**: A wrapped `MessageSync` event with a sequence number (`seq`).
- **Ready**: Sent after successful authentication.
- **Resumed**: Sent after a successful resume.
- **Reconnect**: Sent when the server needs the client to reconnect.
- **Error**: An error occurred.

### sync events (`MessageSync`)

`MessageSync` contains the actual data for updates. Some common events include:

- **Ambient**: Initial state for the user (sent once after `Ready`).
- **RoomCreate / RoomUpdate / RoomDelete**
- **ChannelCreate / ChannelUpdate / ChannelDelete**
- **MessageCreate / MessageUpdate / MessageDelete**
- **RoomMemberCreate / RoomMemberUpdate / RoomMemberDelete**
- **RoleCreate / RoleUpdate / RoleDelete**
- **PresenceUpdate**: A user's presence changed.
- **VoiceState**: A user joined or left a voice channel.
- **DocumentEdit**: Someone edited a document you're subscribed to.
- **MediaProcessed**: A media upload finished processing.
