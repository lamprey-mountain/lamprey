# architecture

todo

## terminology

- **voice state**: a connection to a voice channel
- **sfu**: selective forwarding unit

## types

- `Sfu`: main entrypoint, one single selective forwarding unit
- `Peer`: an endpoint that rtc media can be sent to and received from
- `PeerWebrtc`: a webrtc connection
- `PeerCascading`: a connection to another sfu (cascading trees)
