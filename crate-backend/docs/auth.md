> [!warning]
> this is a work in progress!

| auth method             | identify | 2fa | sudo |
| ----------------------- | -------- | --- | ---- |
| oauth                   | yes      | yes | yes  |
| email magic link        | yes      | no  | no   |
| email password reset    | yes      | no  | yes  |
| totp [^1]               | no       | yes | yes  |
| totp recovery code [^1] | no       | yes | no   |
| email & password        | yes      | no  | yes  |
| captcha [^1]            | no       | no  | no   |
| webauthn [^1]           | yes      | yes | yes  |

- **identify**: if this can be used as the first factor, to get the user id
- **mfa**: if this can be used as the second factor
- **sudo**: if this can be used to enter sudo mode

[^1]: not yet implemented

## registering

1. create a session
2. create a guest account
3. add an auth method, like oauth or email/password
4. use a server invite.

## logging in

1. create a session
2. use an auth method, like oauth or email/password
