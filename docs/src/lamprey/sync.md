# sync

lamprey sync keeps a client's local copy of server state up to date. kerosene
implements two transports:

- **websocket**: the connection-oriented v1 protocol.
- **webtransport**: v1 over multiple QUIC streams, and an experimental v2
  stream protocol.

## connecting

`GET /api/v1/server/@self` returns the endpoints the client should use.
`sync_url` is the websocket URL. `features.webtransport` contains the
webtransport `sync_url` and the server's certificate hash.

### websocket

Open a websocket to `sync_url`, which is `/api/v1/sync` by default:

```
wss://chat.celery.eu.org/api/v1/sync?version=1&format=json
```

websockets only implement v1; use `version=1`. the websocket transport
currently ignores the `version` parameter and always speaks v1.

### webtransport

webtransport is served from a separate QUIC endpoint, over UDP. it is enabled
when the server is built with the webtransport feature and the `[webtransport]`
config section exists; otherwise it is disabled. the default port is `4433` and
the only accepted path is `/api/v1/sync-webtransport`:

```
https://chat.celery.eu.org:4433/api/v1/sync-webtransport?version=1&format=json
```

the server uses a self-signed certificate. clients should pass the sha-256
certificate hash from `features.webtransport.certificate_hashes` as a Web API
`serverCertificateHashes` value. the hash is unpadded url-safe base64 of the
certificate DER. it changes when the server restarts, so re-fetch server info
instead of caching it forever.

### query parameters

| parameter     | values            | notes                                                        |
| ------------- | ----------------- | ------------------------------------------------------------ |
| `version`     | `1`, `2`          | websocket only supports `1`; webtransport supports both      |
| `format`      | `json`, `msgpack` | defaults to `json`                                           |
| `compression` | `deflate`         | zlib-wrapped deflate; v1 and json only; ignored by v2        |

compressed v1 traffic is a continuous deflate stream across frames, compatible
with the web `DecompressionStream("deflate")`. it may be tricky to work with,
since there isn't any framing for v1 websockets; you have to find json/msgpack
boundaries yourself.

## v1

v1 has a single logical connection with one global event stream. webtransport
can additionally multiplex dedicated streams onto the same logical connection.

### framing

#### websocket

- JSON messages are websocket text frames.
- msgpack messages are binary frames.
- when compression is enabled, both directions are binary deflate streams.

#### webtransport

- every message is binary, framed with a 4-byte big-endian length followed by
  the payload.
- incoming frames larger than 1 MiB are rejected by the server.
- JSON or msgpack is used inside each frame. data can also be decompressed with
  `DecompressionStream`.

the first bidirectional stream is the main sync stream. it must start with a
`Hello`. after the connection is authenticated, additional bidirectional
streams can be opened. each one starts with one of these `MessageClient`
messages instead of a `Hello`:

- `DocumentSubscribe`
- `VoiceConnect`
- `MemberListSubscribe`
- `ScriptSubscribe`
- `RoomSubscribe`
- `ChannelSubscribe`

`RoomSubscribe` and `ChannelSubscribe` are only valid on webtransport; sending
them on the main websocket stream is an error. the main stream must stay open
for the connection to stay alive.

### handshake

the client has 5 seconds to send the first message:

```json
{ "type": "Hello", "token": "...", "presence": null, "resume": null }
```

- `token` is a session token. unauthenticated/guest sessions are accepted.
- sessions waiting for a second factor (`Bound`) are rejected.
- `presence` is optional. it is only applied when provided.
- `resume` is `{ "conn": "...", "seq": 123 }` when resuming.

for a new connection the server replies with `Ready`:

```json
{
  "op": "Ready",
  "user": { "...": "..." },
  "application": null,
  "session": { "...": "..." },
  "conn": "...",
  "seq": 1
}
```

for users (not guests (TODO: verify how the old guest system interacts)) it then sends:

1. an `Ambient` sync event with rooms, roles, channels, threads, room members,
   and config.
2. current typing indicators.
3. current voice states.
4. active flume deltas.

### resuming

`seq` is a per-connection sequence number, not a channel or room sequence
number. the first non-ephemeral event, `Ready`, has seq `1`; each subsequent
`Sync` event increments it. seq `0` means nothing has been received yet.

if a transport closes uncleanly, the server keeps the connection for 60 seconds.
reconnect and send `Hello` with `resume` to attach a new transport. the server
rewinds its queue and replays up to the last 256 events after `resume.seq`.

the protocol defines a `Resumed` payload, but kerosene does not currently send
it. a successful resume is just the replayed `Sync` events. if the connection is
gone, the server sends an `Error` followed by
`Reconnect { "can_resume": false }` and closes. if `seq` is too old, attaching
the transport fails and the connection is closed; the protocol intends an
`InvalidSeq` error here, but kerosene does not send one yet.

a clean websocket close (`1000`/`1001`) tears the connection down immediately
instead of keeping it for resume.

### heartbeat

the server sends `Ping` every 30 seconds on each active stream. the client must
reply with `Pong` within 10 seconds. failing to do so closes that stream; losing
the main sync stream tears down the whole connection.

### messages

#### client to server (`MessageClient`)

- `Hello`: authenticate and optionally resume.
- `Presence`: update presence.
- `Pong`: heartbeat response.
- `VoiceConnect`: start a voice connection.
- `VoiceDispatch`: send a signalling command to a voice connection.
- `MemberListSubscribe`: subscribe to member-list ranges.
- `DocumentSubscribe`: subscribe to a document branch.
- `DocumentEdit`: send a document update; the client must be subscribed and
  have `DocumentEdit` permission.
- `DocumentPresence`: update document cursor presence.
- `ScriptSubscribe`: subscribe to a script channel.
- `Subscribe`: replace member-list, document, and script subscriptions. each
  field is optional and holds at most 8 entries.
- `RoomSubscribe` / `ChannelSubscribe`: open a dedicated webtransport stream.

#### server to client (`MessagePayload`)

- `Ping`: heartbeat request.
- `Sync`: a `MessageSync` event with its connection `seq` and optional `nonce`.
- `Error`: an error string and optional `SyncErrorCode`.
- `Ready`: initial user/application/session/connection information.
- `Resumed`: defined, but not currently sent by kerosene.
- `Reconnect`: disconnect and reconnect; `can_resume` says whether the same
  connection can be attached again.

`Ping`, `Error`, `Reconnect`, and `Resumed` are ephemeral: they have no sequence
number and are not kept for resume.

`SyncErrorCode::InvalidSeq` and `ConnectionExpired` force a new connection.
`Timeout`, `InvalidData`, and `TooBig` close the stream.

### subscriptions

most room and channel events are forwarded to members on the main stream. for
lurkers (non-members who can view a room), room and channel events are only sent
on a matching `RoomSubscribe` or `ChannelSubscribe` webtransport stream, which
is why those streams are webtransport-only.

document, voice, member-list, and script subscriptions can also use a dedicated
webtransport stream. on a websocket they use the corresponding `MessageClient`
message on the main stream.

### sync events (`MessageSync`)

`Sync.data` contains a `MessageSync` variant. common events include:

- **Ambient**: initial state for the user.
- **RoomCreate / RoomUpdate / RoomDelete**
- **ChannelCreate / ChannelUpdate / ChannelDelete**
- **MessageCreate / MessageUpdate / MessageDelete**
- **RoomMemberCreate / RoomMemberUpdate / RoomMemberDelete**
- **RoleCreate / RoleUpdate / RoleDelete**
- **PresenceUpdate**
- **VoiceState**
- **DocumentEdit / DocumentPresence**
- **ChannelTyping**
- **FlumeDelta**
- **MediaProcessed**

TODO: short description of the above events, link to some rustdoc that has the
full event list

events are filtered by permissions before queueing, and user-specific fields are
stripped where needed (for example, other users' voice session ids and audit-log
metadata).

## v2

v2 is an experimental, stream-based protocol that is only available over
webtransport. it is not feature-complete yet.

### framing and handshake

request `version=2`. the first bytes of every bidirectional stream are a single
header byte, followed by length-delimited frames (4-byte big-endian length, then
JSON or msgpack).

the first stream MUST use the `Hello` header (`0x00`) and send an `Identify`
initial payload:

```json
{
  "op": "Identify",
  "token": "...",
  "presence": null,
  "properties": {
    "platform": "web",
    "library": "...",
    "application": "..."
  }
}
```

the server replies with `Ready`, then an `Ambient` dispatch. `Resume` and
`Shard` initial messages exist in the protocol but are not implemented. the
server waits up to 15 seconds for authentication.

the hello stream uses the same heartbeat timing as v1. client commands are
`Presence`, `Pong`, and `Close`. server events are `Ping`, `Ready`, `Dispatch`,
`Resumed`, and `Goodbye`; only `Ping`, `Ready`, and `Dispatch` are currently
sent.

### streams

after the hello stream is authenticated, additional streams can be opened. each
starts with a header byte and an initial frame:

| header | stream      | initial                                      | status        |
| ------ | ----------- | -------------------------------------------- | ------------- |
| `0x10` | member list | `{ target, ranges }`                         | implemented   |
| `0x11` | channel     | `{ channel_id, seq? }`                       | implemented   |
| `0x12` | room        | `{ room_id, seq? }`                          | implemented   |
| `0x13` | user        |                                              | unimplemented |
| `0x14` | redex       |                                              | unimplemented |
| `0x15` | invite      |                                              | unimplemented |
| `0x20` | voice       | `VoiceStateUpdate`                           | implemented   |
| `0x30` | document    | `{ document_id, branch_id?, state_vector? }` | implemented   |
| `0x40` | flume       | `{ channel_id, message_id }`                 | implemented   |

- **member list**: sync a slice (or multiple) of a channel or thread's member list (NOTE: except dms/gdms, this may be changed in the future)
- **channel**: sync flumes (live updating messages) for a channel
- **room**: sync typing indicators for channels in a room
- **voice**: connect to a voice channel and manage your rtc state
- **document**: send/recv edits and presence for documents
- **flume**: a more efficient flume update protocol than spamming http requests

### current status

- resume is not implemented (you will be able to resume rooms and channels individually).
- only a few sync events are converted into v2 dispatches properly.
- `compression` isn't implemented yet.
- dispatch sequence numbers are always `0`.
- error and goodbye events are not currently sent; failures close the stream.
