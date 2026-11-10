# voice

lamprey uses WebRTC for voice, video, and screensharing. media does not flow
through the api server: clients connect to an sfu and the api server only
coordinates state and signalling.

## architecture

- **`crate-voice`** is the sfu. it uses `str0m` for WebRTC and runs one shard
  per worker (`config.voice.workers`, or one per cpu by default).
- each voice channel is a **call**. calls are created when the first user
  joins and deleted when the last user leaves (broadcast calls are kept).
- an sfu can **cascade** to other sfus over a QUIC mesh (`crate-voice/src/mesh`)
  so users in the same call can be served by different sfus. (note: not
  implemented yet)
- the api server (`ServiceVoice`) is the master. it tracks calls, voice states,
  sfu health, and allocates users to sfus.

sfus connect to the api server as clients over a websocket (note, this is
internal so clients probably dont need to know how sfu messaging works):

```
ws(s)://{api_url}/api/v1/internal/rpc
Authorization: Server {config.voice.token}
```

the api server sends JSON `SfuCommand`s (`Init`, `CreatePeer`, `Signalling`,
`CreateCascade`, ...) and the sfu sends back `SfuEvent`s (`VoiceDispatch`,
`VoiceState`, `PeerDisconnect`, `Stats`, `CascadePrepared`, ...). `Init`
assigns the sfu its id. sfus are not directly authenticated by clients.

## signalling flow

1. **join**: the client sends a `VoiceConnect` message with a
   `VoiceStateUpdate` on its sync connection. over webtransport this is instead
   a dedicated voice stream whose initial message is the same update.
2. the backend checks that the user can view the channel and that it is
   unlocked and not archived. it tears down the user's previous voice states
   (bots may keep one per channel), creates the call if needed, and allocates an
   sfu.
3. the backend sends `CreatePeer` and the initial voice state to the sfu.
4. **negotiate**: the client sends `SignallingCommand`s (`Offer`, `Answer`,
   `Candidate`, `Subscribe`, `VoiceState`, `Disconnect`). the backend forwards
   them to the sfu as `SfuCommand::Signalling`.
5. the sfu replies with `SfuEvent::VoiceDispatch`. the backend delivers it to
   the target user as a `MessageSync::VoiceDispatch` sync event (or as an event
   on the webtransport voice stream).

the sfu may also send `SignallingEvent::Tracks` when tracks are published or
removed, and `SignallingEvent::Migrate` when the user's call is moved to a
different sfu.

## transports

- **websocket sync (v1)**: `MessageClient::VoiceConnect`,
  `MessageClient::VoiceDispatch`, and `MessageSync::VoiceDispatch` /
  `MessageSync::VoiceState`.
- **webtransport (v2)**: a `Voice` stream (`StreamHeader::Voice`). its
  `Initial` is a `VoiceStateUpdate`; `Command` is a `SignallingCommand`; `Event`
  is a `SignallingEvent`.

closing a voice stream without sending `Disconnect` cleans up the voice state.

## voice state

`VoiceState` is per user, per channel:

- `user_id`, `channel_id`, and optional `room_id`
- `session_id` and `connection_id` (only visible to the owner)
- `joined_at`
- `mute` / `deaf` – set by moderators
- `self_mute` / `self_deaf` – set by the client
- `self_video`
- `screenshare` – optional `started_at` and thumbnail media id
- `suppress` – a transient mute used by broadcast channels and afk channels
- `requested_to_speak_at` – broadcast "raise hand" timestamp

`VoiceStateFull` adds the `User`, optional `RoomMember`, and optional
`ThreadMember`. `VoiceState.muted()` is true when `mute`, `self_mute`, or
`suppress` is set; `deafened()` is true when `deaf` or `self_deaf` is set.

connection limits:

- a user has at most one voice state per channel.
- non-bots have at most one voice state across all channels.
- bots can have any number of voice states.

joining a broadcast channel automatically sets `suppress`, so new participants
are listeners until a moderator lets them speak. joining the room's
`afk_channel_id` also suppresses.

## calls

`Call` is the active state of a voice/broadcast channel. it is created
automatically and has a `topic` that can be changed with `CallUpdate`. a call in
a broadcast channel can also have a `schedule_id` pointing at a calendar
channel, and an `audience_count`.

after a voice state change, the backend broadcasts `MessageSync::VoiceState`
with both the new state and the previous state. after a disconnect it sends
`VoiceDispatch` with `SignallingEvent::Disconnected` and then a `VoiceState`
event with a `null` state.

## permissions

what the backend currently enforces:

- **joining**: `ChannelView` and an unlocked, unarchived channel. there is no
  `VoiceConnect` permission.
- **move a user** (`VoiceStatePatch.channel_id`, move routes, force-ending a
  call): `VoiceMove`, plus view permission on the target channel. moving is
  only allowed inside the same room.
- **disconnect**: disconnecting yourself only needs view permission;
  disconnecting someone else needs `VoiceMove`.
- **moderator mute/deaf/suppress**: both `VoiceMute` and `VoiceDeafen`.
- **request to speak**: users can only set `requested_to_speak_at` for
  themselves, and need `VoiceRequest`.
- **call topic**: `CallUpdate`.
- **priority speaker**: `VoicePriority` is sent to the sfu and enables priority
  audio if the user is not muted.

`VoiceSpeak`, `VoiceVideo`, `VoiceVad`, and `VoiceBroadcast` exist in the
permission enum and default role sets, but the current backend and sfu do not
enforce them. speaking is gated by `mute`/`self_mute`/`suppress`, and video is
gated by `self_video` and `suppress`.

voice states are also visible through the rest api under `/api/v1/voice`, with
routes for listing states, patching moderator state, moving, disconnecting, and
reading/updating calls and rings.
