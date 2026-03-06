# voice

lamprey mountain uses WebRTC for low-latency voice and video communication.

## architecture

- **crate-voice**: A standalone SFU (Selective Forwarding Unit) that handles
  WebRTC media streams.
- **Signalling**: Done via the main WebSocket sync connection.
- **Voice State**: Managed by the backend to keep track of who is in which
  channel and their status (mute/deaf).

## signalling flow

1. **Join**: The client joins a voice channel by updating voice state.
2. **Connect**: The backend provides the client with the voice server's address
   and a token.
3. **Negotiate**: The client and the voice server exchange WebRTC SDP
   offers/answers via `VoiceDispatch` messages on the main WebSocket.
   - Client sends `VoiceDispatch` to backend.
   - Backend forwards it to `crate-voice`.
   - `crate-voice` responds to backend.
   - Backend sends `VoiceDispatch` back to client via `Sync`.

## voice state

Each user in a voice channel has a `VoiceState`:

- `channel_id`: The channel the user is currently in.
- `self_mute` / `self_deaf`: Client mute/deaf status.
- `mute` / `deaf`: Server-side mute/deaf status.
- `suppress`: Whether the user is suppressed (e.g. in a broadcast channel).
- `requested_to_speak_at`: When the user requested to speak (for broadcast
  channels).
- There are a few more properties, see rustdoc

## permissions

- `VoiceConnect`: Permission to join a voice channel.
- `VoiceSpeak`: Permission to transmit microphone audio.
- `VoiceVideo`: Permission to transmit camera video and screenshare. Screenshare
  audio is tied to this permission, not `VoiceSpeak`.
- `VoiceMute` / `VoiceDeafen`: Administrative permissions to mute/deaf others.
- `VoiceMove`: Permission to move users between voice channels.
- `VoiceDisconnect`: Permission to disconnect users from a voice channel.
