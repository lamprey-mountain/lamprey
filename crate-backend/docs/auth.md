> [!warning]
>
> this isn't implemented yet!

| auth method              | identify | register | 2fa | sudo    |
| ------------------------ | -------- | -------- | --- | ------- |
| oauth                    | yes      | yes      | yes | yes[^1] |
| email magic link         | yes      | yes      | no  | no      |
| email password reset[^2] | yes      | yes      | no  | yes     |
| totp                     | no       | no       | yes | yes     |
| password                 | yes      | yes      | no  | yes[^1] |
| captcha                  | no       | no       | no  | no      |
| webauthn                 | yes      | yes      | yes | yes     |

- **register**: if a new user can be registered via this auth method
- **identify**: if this can be used as the first factor, to get the user id
- **mfa**: if this can be used as the second factor
- **sudo**: if this can be used to enter sudo mode

[^1]: appears as "sign up with email"

[^2]: only if there is no other option. password is preferred over oauth.

oauth or email or password or webauthn
