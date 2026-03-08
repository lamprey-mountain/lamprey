import { Session } from "sdk";
import { BaseService } from "../core/Service";

export class SessionsService extends BaseService<Session> {
	getKey(item: Session): string {
		return item.id;
	}

	async fetch(id: string): Promise<Session> {
		// No endpoint to fetch a single session by ID currently
		throw new Error("Method not implemented.");
	}

	async deleteSession(session_id: string) {
		const { error } = await this.client.http.DELETE(
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
		// If we delete the current session, we should probably log out?
		// But that logic might be handled by event listener or UI.
		// For now just remove from cache if we had it.
		this.delete(session_id);
	}
}
