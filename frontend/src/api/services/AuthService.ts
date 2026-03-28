import { Session } from "sdk";
import { BaseService } from "../core/Service";

export class AuthService extends BaseService<never> {
	protected cacheName = "auth";

	getKey(item: never): string {
		throw new Error("AuthService does not cache items");
	}

	async fetch(id: string): Promise<never> {
		throw new Error("AuthService does not fetch items");
	}

	async oauthUrl(provider: string): Promise<string> {
		const result = await this.retryWithBackoff<any>(() =>
			this.client.http.POST("/api/v1/auth/oauth/{provider}", {
				params: {
					path: { provider },
				},
			})
		);
		return result.data.url;
	}

	async passwordLogin(
		body: { email: string; password: string; type: "Email" },
	): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.POST("/api/v1/auth/password", {
				body: body as any,
			})
		);
	}

	async createTempSession(): Promise<Session> {
		return await this.retryWithBackoff(() =>
			this.client.http.POST("/api/v1/session", {
				body: {},
			})
		);
	}
}
