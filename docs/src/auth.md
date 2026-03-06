# auth

lamprey supports several authentication methods.

| auth method            | identify | 2fa | sudo | status        |
| ---------------------- | -------- | --- | ---- | ------------- |
| oauth (discord/github) | yes      | yes | yes  | implemented   |
| email magic link       | yes      | no  | no   | implemented   |
| email password reset   | yes      | no  | yes  | implemented   |
| totp                   | no       | yes | yes  | implemented   |
| totp recovery code     | no       | yes | no   | implemented   |
| email & password       | yes      | no  | yes  | implemented   |
| captcha                | no       | no  | no   | unimplemented |
| webauthn               | yes      | yes | yes  | unimplemented |

- **identify**: if this can be used as the first factor to log in.
- **2fa**: if this can be used as the second factor.
- **sudo**: if this can be used to enter sudo mode.

## sudo mode

Certain sensitive operations (e.g. changing password, deleting account) require
**sudo mode**. You can enter sudo mode by providing a second factor or by using
specific auth methods. Sudo mode typically expires after 5 minutes.

## flows

### registering

1. create a session (unauthenticated).
2. create a guest account (optional, usually happens automatically on some
   flows).
3. add an auth method (OAuth, email/password).
4. use a server invite to finish registration.

### logging in

1. create a session.
2. provide an identification factor (OAuth, email magic link, or
   email/password).
3. if 2FA is enabled, provide the second factor (TOTP).
4. if successful, the session becomes `Authorized`.
