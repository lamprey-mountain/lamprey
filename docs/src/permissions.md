# permissions

**OUTDATED DOCS!** some of this stuff may work, but a lot may not.

how permissions are calculated and applied in lamprey.

## room permissions

1. if the user is the **owner** or the **server user**, they have all permissions (and maximum rank).
2. start with an empty permission set.
3. add all **allow** permissions from the roles the user has (including the `@everyone` role).
4. if the user has the **Admin** permission, return all permissions.
5. remove all **deny** permissions from the roles the user has (including the `@everyone` role).

## channel permissions

1. start with the user's **room permissions**.
2. if the user has **Admin**, return all permissions.
3. apply **overwrites** for the parent channel (if any), then for the current
   channel.

overwrites are applied in this order:

1. `@everyone` role **allow**.
2. `@everyone` role **deny**.
3. Role **allows**.
4. Role **denies**.
5. User **allows**.
6. User **denies**.

## restrictions

Specific states can override the final permissions.

View permissions are `ViewChannel`, `ViewAuditLog`, and `ViewAnalytics`.

- **Timed Out**: remove all permissions except view permissions.
- **Quarantined**: remove all permissions except view permissions and
  `MemberNickname`.
- **Lurker** (non-members in a public room): remove all permissions except view
  permissions.
  - In broadcast channels, members will retain `VoiceConnect`, `VoiceRequest`,
    and `VoiceVad`

## hierarchy (rank)

- a member's **rank** is the highest `position` of all roles they possess.
- room owners have a special "infinite"/maximum rank.
- rank restricts administrative actions:
  - you can only manage members with a lower rank than yours (kick, ban,
    timeout).
  - you can only apply or reorder roles with a lower position than your rank.

## channel locks

Channels and threads can be **locked**.

- when a channel is locked, normal message creation is restricted.
- users with `ThreadManage`, `ChannelManage`, or `ThreadLock` can also bypass
  the lock.
- specific roles can be configured to bypass the lock. (this is api only for now)
- the lock can be set to expire at a certain time. (this is api only for now)

## notes

The `Admin` permission overrides all denies and cannot be revoked. This means
all private channels are visible to admins. Use with caution!
