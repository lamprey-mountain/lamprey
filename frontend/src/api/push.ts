import type { PushCreate, PushInfo } from "sdk";
import type { Api } from "../api.tsx";

export class Push {
	public api: Api = null as unknown as Api;

	async register(body: PushCreate): Promise<PushInfo> {
		const { data, error } = await this.api.client.http.POST(
			"/api/v1/push",
			{
				body,
			},
		);
		if (error) {
			console.error(error);
			throw new Error(error);
		}
		return data as PushInfo;
	}

	async delete(): Promise<void> {
		const { error } = await this.api.client.http.DELETE(
			"/api/v1/push",
		);
		if (error) {
			console.error(error);
			throw new Error(error);
		}
	}

	async get(): Promise<PushInfo> {
		const { data, error } = await this.api.client.http.GET(
			"/api/v1/push",
		);
		if (error) {
			console.error(error);
			throw new Error(error);
		}
		return data as PushInfo;
	}
}
