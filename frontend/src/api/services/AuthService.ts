import type { Session } from "sdk";
import { BaseService } from "../core/Service";

export class AuthService extends BaseService<never> {
	protected cacheName = "auth";

	getKey(_item: never): string {
		throw new Error("AuthService does not cache items");
	}

	async fetch(_id: string): Promise<never> {
		throw new Error("AuthService does not fetch items");
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
		// // After successful login, restart the sync connection to get the updated session
		// this.client.stop();
		// const token = localStorage.getItem("token");
		// if (token) {
		// 	this.client.start(token);
		// }
	}

	async createSession(): Promise<Session> {
		return await this.retryWithBackoff(() =>
			this.client.http.POST("/api/v1/session", {
				body: {},
			}),
		);
	}
}
