import type { PreferencesRoom } from "sdk";
import { useApi, useRooms } from "@/api";
import { CheckboxOption } from "@/atoms/CheckboxOption";
import { Checkbox } from "@/atoms/icons";
import { Modal } from "./mod";

interface ModalPrivacyProps {
	room_id: string;
}

type RoomPrivacy = PreferencesRoom["privacy"];
type RoomPrivacyKey = keyof RoomPrivacy;

const DEFAULT_PRIVACY: RoomPrivacy = {
	dms: true,
	friends: true,
	rpc: true,
	exif: true,
};

const DEFAULT_NOTIFS: PreferencesRoom["notifs"] = {
	mention_everyone: true,
	mention_roles: true,
	messages: null,
	mute: null,
	replies: null,
	threads: null,
};

export const ModalPrivacy = (props: ModalPrivacyProps) => {
	const api = useApi();
	const rooms2 = useRooms();
	const room = rooms2.use(() => props.room_id);

	const privacy = (): RoomPrivacy =>
		room()?.preferences?.privacy ?? DEFAULT_PRIVACY;

	const setPrivacy = (key: RoomPrivacyKey, value: boolean) => {
		const r = room();
		if (!r) return;

		const current = r.preferences;
		const newPreferences: PreferencesRoom = {
			notifs: current?.notifs ?? DEFAULT_NOTIFS,
			frontend: current?.frontend ?? {},
			privacy: { ...(current?.privacy ?? DEFAULT_PRIVACY), [key]: value },
		};

		rooms2.cache.set(r.id, { ...r, preferences: newPreferences });
		api.client.http.PUT("/api/v1/preferences/room/{room_id}", {
			params: { path: { room_id: r.id } } as any,
			body: newPreferences as any,
		});
	};

	return (
		<Modal>
			<div class="modal-privacy">
				<h3>privacy</h3>
				<CheckboxOption
					id="opt-dms"
					checked={privacy().dms}
					onChange={(checked) => setPrivacy("dms", checked)}
					seed={`modal-privacy-${props.room_id}-dms`}
				>
					<Checkbox
						checked={privacy().dms}
						seed={`modal-privacy-${props.room_id}-dms`}
					/>
					<label for="opt-dms">
						<div>Allow direct messages</div>
						<div class="dim">
							Let others send direct messages to you. Bots, moderators, and
							friends can always start dms.
						</div>
					</label>
				</CheckboxOption>
				<CheckboxOption
					id="opt-friends"
					checked={privacy().friends}
					onChange={(checked) => setPrivacy("friends", checked)}
					seed={`modal-privacy-${props.room_id}-friends`}
				>
					<Checkbox
						checked={privacy().friends}
						seed={`modal-privacy-${props.room_id}-friends`}
					/>
					<label for="opt-friends">
						<div>Allow friend requests</div>
						<div class="dim">Let others send friend requests to you.</div>
					</label>
				</CheckboxOption>
				<CheckboxOption
					id="opt-rpc"
					checked={privacy().rpc}
					onChange={(checked) => setPrivacy("rpc", checked)}
					seed={`modal-privacy-${props.room_id}-rpc`}
				>
					<Checkbox
						checked={privacy().rpc}
						seed={`modal-privacy-${props.room_id}-rpc`}
					/>
					<label for="opt-rpc">
						<div>Share rich presence</div>
						<div class="dim">
							Share rich presence with everyone else in this room. Friends can
							always view your rich presence.
						</div>
					</label>
				</CheckboxOption>
				<CheckboxOption
					id="opt-exif"
					checked={!privacy().exif}
					onChange={(checked) => setPrivacy("exif", !checked)}
					seed={`modal-privacy-${props.room_id}-exif`}
				>
					<Checkbox
						checked={!privacy().exif}
						seed={`modal-privacy-${props.room_id}-exif`}
					/>
					<label for="opt-exif">
						<div>Strip exif metadata</div>
						<div class="dim">
							Strip potentially sensitive exif metadata (ie. camera model or
							location) from images you upload.
						</div>
					</label>
				</CheckboxOption>
			</div>
		</Modal>
	);
};
