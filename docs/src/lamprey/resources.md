# resources

a high-level map of lamprey's main types. most live in
`crate-common/src/v1/types` (a few in `v2/types`).

## identifiers

resources use typed uuidv7 ids: `RoomId`, `ChannelId`, `MessageId`, `UserId`,
`RoleId`, `MediaId`, `EmojiId`, `TagId`, `ApplicationId`, `RedexId`, and so on.
many mutable resources also carry a `version_id` that changes when they change.

## users

- `User`: an account, bot, webhook user, or bridge puppet. has profile fields,
  `bot`/`system` flags, optional suspension, presence, and auth connections.
- `Session`: an authenticated login session with a status
  (`Unauthorized`/`Bound`/`Authorized`/`Sudo`), expiry, and device imprint.
- `Relationship`: friend requests, friendships, and blocks between users.
- `Connection`: an oauth application authorized by a user.

## rooms

`Room` is a collection of members, channels, roles, and audit logs. it also acts
as the main authorization context: room roles and permissions are the base for
channel permissions. the default room type contains the usual channels, but
channels do not have to belong to a room (dms and gdms do not).

- `Room`: name, icon/banner, owner, counts, public/archived/deleted state,
  welcome and afk channel settings, security requirements, and feature flags.
- `RoomType`: `Default`, `Server`, or `Emoji` (a pack room).
- `RoomFeature`: opt-in `Scripts`, `Automod`, `Documents`, `Vanity`, and `Llm`.
- `RoomMember`: a user's membership in a room: roles, nickname/description
  overrides, mute/deaf, timeout, quarantine, and join time.
- `Role`: a named allow/deny permission set with a hierarchy `position`, plus
  hoist, sticky, self-applicable, and mentionable flags.

see [permissions](./permissions.md) for how these combine.

## channels

`Channel` covers text channels, threads, dms, gdms, voice, categories, forums,
and the document/calendar/script features. it has a `ChannelType`, an optional
room and parent, name/description, position, permission overwrites, lock state,
unread state, and slowmode.

`ChannelType`:

- `Text`: normal instant-messaging channel.
- `Announcement`: text channel with announcement-oriented behavior.
- `ThreadPublic`: thread visible to anyone who can see the parent.
- `ThreadPrivate`: thread visible only to its members and `ThreadManage` users.
- `ThreadForum2`: a forum post thread, behaving like a public thread.
- `Dm`: direct message between two users.
- `Gdm`: group direct message.
- `Forum`: container for forum post threads.
- `Voice`: voice/video call channel.
- `Broadcast`: one-to-many voice channel where new members start suppressed.
- `Category`: groups channels in the sidebar.
- `Calendar`: channel containing calendar events.
- `Forum2`: tree-style long-form discussion channel.
- `Info`: read-only info channel without messages, voice, or threads.
- `Ticket`: support-ticket container; each ticket is a private thread.
- `Document`: a single document, either standalone or in a wiki.
- `DocumentComment`: comment thread on a document.
- `DocumentBranch`: editing-context thread for a document branch.
- `Wiki`: channel containing multiple documents.
- `Scripts`: channel containing executable scripts (redexes).

`Tag` belongs to a forum-like channel and can be applied to its threads.
`ThreadMember` is the lighter membership relation for threads.

## messages

`Message` is a stable envelope (id, channel, author, timestamps, pins,
reactions, thread) pointing at a `MessageVersion`, which contains the actual
content. edits create new versions, and system events are messages too.

`MessageType`:

- `DefaultMarkdown`: normal markdown message with attachments, embeds, and
  components.
- `ThreadInitial`: the starting message of a thread.
- `Encrypted`: an end-to-end encrypted message (feature-gated).
- `MessagePinned`: a pin/unpin event.
- `MessagesMoved`: messages were moved between channels (feature-gated).
- `MemberAdd`: a user was added to a thread or group dm.
- `MemberRemove`: a user was removed from a thread or group dm.
- `MemberJoin`: a user joined the room.
- `Call`: a dm/gdm call started or ended.
- `ChannelRename`: a channel or thread was renamed.
- `ChannelPingback`: a channel mentioned another channel (incomplete).
- `ChannelMoved`: a thread was moved to another parent.
- `ChannelTagged`: a thread's tags changed.
- `ChannelIcon`: a channel's icon changed.
- `ThreadCreated`: a thread was created from a message.
- `AutomodExecution`: an automod rule matched and ran actions.
- `Nudge`: a user nudged another user.
- `DocumentTag`: a document revision was tagged.
- `DocumentEdits`: a summary of recent document edits.
- `DocumentMerged`: one document branch merged into another.

`MessageAttachment` points at `Media`, `Embed` is a link preview, and
`Reaction` is a unicode, custom emoji, or tag reaction.

## media

`Media` is uploaded content plus processing metadata; see [media](./media.md).
`MediaReference` is a create-time media id, url, or multipart index.
`MediaLinkType` records where media is used (message, version, avatar, banner,
icon, embed, emoji, script, or document). `Track` is an extra subtitle or
caption file attached to media.

## other resources

- `EmojiCustom`: a custom emoji backed by media.
- `Invite`: access to a room, gdm, server registration, or friend request.
- `Application`: a bot or oauth client, optionally with bridge/puppet support.
- `Webhook`: a token that can post messages into a channel.
- `AutomodRule`: room moderation rules and the actions they trigger.
- `AuditLogEntry`: an administrative action record scoped to a room.

`Sync`/`MessageSync` keeps clients updated with creates, updates, deletes,
voice dispatches, and transient events; see [sync](./sync.md).

## less common features

- `Document`, `DocumentBranch`, `DocumentTag`, and `Changeset`: CRDT-backed
  documents, wiki pages, and their history. the documents/wikis feature is
  experimental.
- `CalendarEvent` and `CalendarParticipant`: events, recurrence, and RSVPs in
  calendar channels. the calendar feature is experimental.
- `Redex` and `RedexVersion`: executable scripts and their versions, declared
  capabilities, dependencies, and observability data. the scripts feature is
  experimental.
- `VoiceState`/`Call`: users connected to a voice channel and the channel's
  active call; see [voice](./voice.md).

federation types (`Remote<T>`, `RemoteReq<T>`) exist, but federation is not
implemented yet.
