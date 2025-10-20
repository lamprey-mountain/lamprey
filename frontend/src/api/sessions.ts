import type { Api } from "../api.tsx";

export class Sessions {
	api: Api = null as unknown as Api;

	async delete(session_id: string) {
		const { error } = await this.api.client.http.DELETE(
			"/api/v1/session/{session_id}",
			{
				params: {
					path: {
						session_id,
					},
				},
			},
		);
		if (error) throw error;
	}
}
