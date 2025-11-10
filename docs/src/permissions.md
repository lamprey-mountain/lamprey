# permissions

how permissions work in lamprey

## calculating

resolving overwrites for a room

1. if the user is the owner, they have all permissions
2. add all allow permissions for roles
3. if the user has Admin, return all permissions
4. remove all deny permissions for roles

resolving overwrites for a channel

1. start with parent_id channel (or room) permissions
2. if the user has Admin, return all permissions
3. add all allow permissions for everyone
4. remove all deny permissions for everyone
5. add all allow permissions for roles
6. remove all deny permissions for roles
7. add all allow permissions for users
8. remove all deny permissions for users

timed out

1. remove all permissions except ViewChannel and ViewAuditLog

<!--
guest/lurker

1. remove all permissions except ViewChannel, ViewAuditLog, and VoiceConnect
2. if a channel isnt Broadast, remove VoiceConnect
-->

## hierarchy

- a member's rank is the position of their highest role.
- you can only reorder and edit roles with a lower position than your rank
- you can only apply roles to members with a lower position than your rank (but
  you can apply those roles to anyone, regardless of rank)
- you can only kick, ban, and timeout members with a lower rank than you
