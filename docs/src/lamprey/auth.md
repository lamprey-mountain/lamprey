# auth

kerosene implements the authentication methods below. the `status` column
describes the kerosene implementation, not the lamprey protocol.

| auth method                 | identify      | 2fa | sudo        | status      |
| --------------------------- | ------------- | --- | ----------- | ----------- |
| oauth (discord/github/oidc) | yes           | no  | no          | implemented |
| email magic link            | yes           | no  | no          | implemented |
| email password reset        | yes           | no  | yes[^reset] | implemented |
| totp                        | no            | yes | yes         | implemented |
| totp recovery code          | no            | yes | yes         | implemented |
| email & password            | yes           | no  | no          | implemented |
| webauthn                    | no[^webauthn] | yes | yes         | implemented |
| captcha                     | no            | no  | no          | stub        |

- **identify**: can be used as the first factor on an unauthenticated session.
- **2fa**: can complete a session that is waiting for a second factor.
- **sudo**: can be used to enter sudo mode.
- **stub**: the routes exist but return empty responses.

[^reset]: a reset link enters sudo directly only when MFA is disabled. otherwise it binds the session and a sudo-capable second factor must be completed.

[^webauthn]: kerosene does not implement usernameless/discoverable authentication. a webauthn ceremony only works for a session that is already bound to the user by another factor.

## sessions

a session is always in one of four states (`SessionStatus`):

- `Unauthorized`: no user attached, can only authenticate.
- `Bound { user_id, expires_at }`: the first factor succeeded, but a second
  factor is still required. the session has 15 minutes to complete it; after
  that a new login is required.
- `Authorized { user_id }`: normal authenticated session.
- `Sudo { user_id, sudo_expires_at }`: authenticated and elevated. sudo mode
  lasts 5 minutes.

`Bound` and `Sudo` deadlines are stored in the session, and sudo expiry is
checked on every `ensure_sudo` call.

## mfa

a user has MFA when TOTP is enabled or when they have at least one webauthn
authenticator. oauth, email, and password are not considered second factors.

`GET /auth/state` returns `has_email`, `has_totp`, `has_password`,
`oauth_providers`, and registered `authenticators`.

when removing an auth method, the user must keep at least one way to log
in again: an oauth provider, an email address, or a webauthn authenticator.
passwords and TOTP don't count for this check. even though you can technically
log in with user id and password, i don't expect users to remember their user
id.

## flows

### registering

1. create an unauthenticated session.
2. `POST /user` creates a user. setting an optional invite code will record
   the inviter (`parent_id`), in the future this will be used for antispam. the
   session is immediately set to `Authorized` as the new user without attaching
   any auth method.
3. add auth methods. setting a password, initializing TOTP, and registering a
   passkey all require sudo.

### logging in

1. use a session that is `Unauthorized`, or `Bound` but expired. already
   authenticated sessions are rejected.
2. provide a first factor: oauth, password, email magic link, or a reset link.
3. if the user has MFA, the session becomes `Bound` and a second factor is
   required within 15 minutes. this includes totp and webauthn for now.
4. on success, the session becomes `Authorized` and a login audit log entry
   is recorded. completing a sudo request sets your session state to `Sudo`
   instead.

### sudo mode

sensitive operations require sudo mode (`ensure_sudo`). it lasts 5 minutes.

current ways to enter it:

- `POST /auth/totp/sudo`
- `POST /auth/totp/recovery/sudo`
- `POST /auth/webauthn/sudo/finish`
- a password reset link, when MFA is disabled
- `POST /auth/sudo/upgrade`, a temporary debug endpoint that upgrades any
  authenticated session without verifying a factor

`DELETE /auth/sudo` drops the session back to `Authorized`. server identities
are always sudo, and the static admin token creates a sudo session expiring
after one hour.

(this may be redone in the future)
