# permissions

how lamprey calculates and applies permissions.

the calculator lives in `kerosene-services/src/services/cache/permissions.rs`.
it returns a `Permissions2` value containing the permission bits, rank, member
state, and channel metadata.

## room permissions

1. the **server user** and the room **owner** get `Admin` and the maximum rank
   (`u16::MAX`).
2. a non-member of a **public** room gets the `@everyone` role's allows minus
   its denies. a non-member of a private room gets nothing.
3. a member starts with every role's allows, including `@everyone`.
4. if the resulting allows contain `Admin`, calculation stops: denies,
   timeouts, and quarantines are ignored.
5. otherwise, every role's denies are removed from the permission set.
6. an active `timeout_until` or `quarantined` flag is recorded for later.

role `allow` and `deny` lists are unioned across all of the member's roles
before applying denies, so a deny in one role removes an allow from another.

## channel permissions

channel permissions start from the room result.

1. if the user has `Admin`, channel overwrites, locks, private threads, and
   restrictions are skipped entirely.
2. otherwise, overwrites are applied for each parent channel first, then for
   the channel itself. a child overwrite can therefore undo a parent overwrite.
3. for each channel, overwrites are applied in this order:
   - `@everyone` allow, then `@everyone` deny
   - each of the member's roles' allow, then deny
   - the member's user overwrite allow, then deny
4. channel locks are applied after that channel's overwrites.
5. private threads are visible only to thread members and users with
   `ThreadManage` (or `Admin`). everyone else gets no permissions.

dms and gdms have no room. recipients and the channel owner get the
`EVERYONE_TRUSTED` defaults plus `ChannelView`; everyone else cannot see the
channel.

## restrictions

the following masks are applied after channel overwrites. they can only remove
permissions, never add them.

- **timed out**: keep only view permissions.
- **quarantined**: keep only view permissions and `MemberNickname`.
- **lurker** (non-member): keep only view permissions. in a broadcast channel,
  keep view permissions plus `VoiceRequest` and `VoiceVad`.

view permissions are `ChannelView`, `AuditLogView`, and `AnalyticsView`.

`Admin` bypasses timeout and quarantine. non-members are masked even if they
somehow have `Admin`, because the lurker mask is applied after the admin check.

## hierarchy (rank)

- a member's **rank** is the highest `position` among their roles.
- room owners and the server user have maximum rank.
- `Admin` grants permissions, but does not by itself grant rank.
- you cannot manage a member (kick, ban, timeout) whose rank is equal to or
  higher than yours.
- you can only add, remove, or reorder roles whose position is strictly lower
  than your rank.

## channel locks

channels and threads can be locked to prevent users from interacting with it.
the ui only lets you fully lock/unlock channels, but more granular locking can
be done with a `Locked` value:

- `until` optionally expires the lock.
- `allow_roles` lists roles that bypass the lock.

a lock does not only block message creation: if the user has no bypass, the
channel is treated as if the user were timed out, leaving only view
permissions.

the lock is bypassed by `Admin`, `ChannelManage`, `ThreadManage` in threads, or
by having one of `allow_roles`. an expired lock is ignored.

## enforcement

most routes declare requirements with a `Requirements` value. enforcement
checks, in order:

- suspended users, unless the route allows them
- mfa and sudo requirements from the route and the room's `RoomSecurity`
- oauth scopes
- visibility (`ensure_view` returns `UnknownRoom`/`UnknownChannel` instead of
  leaking existence)
- required permissions
- slowmode bypass, unless the user has `ChannelSlowmodeBypass`,
  `ChannelManage`, `MemberTimeout`, or `ThreadManage` in a thread
- channel lock, unless the route allows locked channels

missing view permission returns a 404-style error. missing other permissions
return `403` with the full list of missing permissions.
