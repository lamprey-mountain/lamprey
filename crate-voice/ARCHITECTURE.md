# architecture

- `Sfu`: main entrypoint, one single selective forwarding unit
- `Peer`: an endpoint that rtc media can be sent to and received from
- `PeerWebrtc`: a webrtc connection
- `PeerCascading`: a connection to another sfu (cascading trees)

todo
