import type { Session } from "sdk";
import { BaseService } from "../core/Service";

/** mirrors `common::v1::types::auth::AuthState` */
export interface AuthState {
	has_email: boolean;
	has_totp: boolean;
	has_password: boolean;
	oauth_providers: string[];
	authenticators: WebauthnAuthenticator[];
}

export interface WebauthnAuthenticator {
	id: string;
	name: string;
	created_at: string;
}

export interface TotpRecoveryCode {
	code: string;
	/** if this is set the code can no longer be used */
	used_at?: string | null;
}

// TODO: drop the path casts once the openapi schema is regenerated with the auth routes
// biome-ignore lint/suspicious/noExplicitAny: routes are missing from the generated schema
const stale = (path: string): any => path;

export class AuthService extends BaseService<never> {
	protected cacheName = "auth";

	getKey(_item: never): string {
		throw new Error("AuthService does not cache items");
	}

	async fetch(_id: string): Promise<never> {
		throw new Error("AuthService does not fetch items");
	}

	/** the auth methods and factors of the current user */
	async state(): Promise<AuthState> {
		return await this.body<AuthState>(() =>
			this.client.http.GET(stale("/api/v1/auth/state"), {}),
		);
	}

	async oauthUrl(provider: string): Promise<string> {
		const result = await this.retryWithBackoff<{ url: string }>(() =>
			this.client.http.POST("/api/v1/auth/oauth/{provider}", {
				params: {
					path: { provider },
				},
			}),
		);
		return result.url;
	}

	async passwordLogin(body: {
		email: string;
		password: string;
		type: "Email";
	}): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.POST("/api/v1/auth/password", {
				body,
			}),
		);
	}

	async createSession(): Promise<Session> {
		return await this.retryWithBackoff(() =>
			this.client.http.POST("/api/v1/session", {
				body: {},
			}),
		);
	}

	// totp

	async totpInit(): Promise<string> {
		const { secret } = await this.body<{ secret: string }>(() =>
			this.client.http.POST(stale("/api/v1/auth/totp/init"), {}),
		);
		return secret;
	}

	async totpEnable(code: string): Promise<void> {
		await this.body<void>(() =>
			this.client.http.POST(stale("/api/v1/auth/totp/enable"), {
				body: { code },
			}),
		);
	}

	async totpDelete(): Promise<void> {
		await this.body<void>(() =>
			this.client.http.DELETE(stale("/api/v1/auth/totp"), {}),
		);
	}

	async totpRecoveryCodes(): Promise<TotpRecoveryCode[]> {
		const { codes } = await this.body<{ codes: TotpRecoveryCode[] }>(() =>
			this.client.http.GET(stale("/api/v1/auth/totp/recovery-codes"), {}),
		);
		return codes;
	}

	async totpRotateRecoveryCodes(): Promise<TotpRecoveryCode[]> {
		const { codes } = await this.body<{ codes: TotpRecoveryCode[] }>(() =>
			this.client.http.POST(stale("/api/v1/auth/totp/recovery-codes"), {}),
		);
		return codes;
	}

	// webauthn

	/** the registration options, as a json string */
	async webauthnRegisterChallenge(): Promise<string> {
		const { challenge } = await this.body<{ challenge: string }>(() =>
			this.client.http.POST(
				stale("/api/v1/auth/webauthn/register/challenge"),
				{},
			),
		);
		return challenge;
	}

	async webauthnRegisterFinish(
		credential: string,
	): Promise<WebauthnAuthenticator> {
		return await this.body<WebauthnAuthenticator>(() =>
			this.client.http.POST(stale("/api/v1/auth/webauthn/register/finish"), {
				body: { credential },
			}),
		);
	}

	async webauthnAuthenticators(): Promise<WebauthnAuthenticator[]> {
		const { authenticators } = await this.body<{
			authenticators: WebauthnAuthenticator[];
		}>(() =>
			this.client.http.GET(stale("/api/v1/auth/webauthn/authenticator"), {}),
		);
		return authenticators;
	}

	async webauthnRename(id: string, name: string): Promise<void> {
		await this.body<void>(() =>
			this.client.http.PATCH(
				stale("/api/v1/auth/webauthn/authenticator/{authenticator_id}"),
				{
					params: { path: { authenticator_id: id } },
					body: { name },
				},
			),
		);
	}

	async webauthnDelete(id: string): Promise<void> {
		await this.body<void>(() =>
			this.client.http.DELETE(
				stale("/api/v1/auth/webauthn/authenticator/{authenticator_id}"),
				{ params: { path: { authenticator_id: id } } },
			),
		);
	}

	/** run a request against a route missing from the generated schema, returning its body */
	private async body<T>(
		fn: () => Promise<{ data?: unknown; response: Response }>,
	): Promise<T> {
		return (await this.retryWithBackoff<unknown>(fn)) as T;
	}
}
