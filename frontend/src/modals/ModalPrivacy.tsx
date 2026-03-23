import { createSignal } from "solid-js";
import { Modal } from "./mod";
import { Checkbox } from "../icons";
import { CheckboxOption } from "../atoms/CheckboxOption";

interface ModalPrivacyProps {
	room_id: string;
}

export const ModalPrivacy = (props: ModalPrivacyProps) => {
	const [dms, setDms] = createSignal(false);
	const [friends, setFriends] = createSignal(false);
	const [rpc, setRpc] = createSignal(false);
	const [exif, setExif] = createSignal(false);

	return (
		<Modal>
			<div class="modal-privacy">
				<h3>privacy</h3>
				<CheckboxOption
					id="opt-dms"
					checked={dms()}
					onChange={setDms}
					seed={`modal-privacy-${props.room_id}-dms`}
				>
					<Checkbox
						checked={dms()}
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
					checked={friends()}
					onChange={setFriends}
					seed={`modal-privacy-${props.room_id}-friends`}
				>
					<Checkbox
						checked={friends()}
						seed={`modal-privacy-${props.room_id}-friends`}
					/>
					<label for="opt-friends">
						<div>Allow friend requests</div>
						<div class="dim">
							Let others send friend requests to you.
						</div>
					</label>
				</CheckboxOption>
				<CheckboxOption
					id="opt-rpc"
					checked={rpc()}
					onChange={setRpc}
					seed={`modal-privacy-${props.room_id}-rpc`}
				>
					<Checkbox
						checked={rpc()}
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
					checked={exif()}
					onChange={setExif}
					seed={`modal-privacy-${props.room_id}-exif`}
				>
					<Checkbox
						checked={exif()}
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
