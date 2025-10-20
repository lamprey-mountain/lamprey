import type { Api } from "../api.tsx";

export class Auth {
	api: Api = null as unknown as Api;

	async oauthUrl(provider: string): Promise<string> {
		const { data, error } = await this.api.client.http.POST(
			"/api/v1/auth/oauth/{provider}",
			{
				params: {
					path: {
						provider,
					},
				},
			},
		);
		if (error) throw error;
		return data.url;
	}

	async passwordLogin(body: any) {
		const { error } = await this.api.client.http.POST("/api/v1/auth/password", {
			body,
		});
		if (error) throw error;
	}
}
