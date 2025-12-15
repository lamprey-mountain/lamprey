import { createSignal } from "solid-js";
import { Modal } from "./mod";
import { Checkbox } from "../icons";

interface ModalPrivacyProps {
	room_id: string;
}

export const ModalPrivacy = (props: ModalPrivacyProps) => {
	const [dms, setDms] = createSignal(false);
	const [rpc, setRpc] = createSignal(false);
	const [exif, setExif] = createSignal(false);

	return (
		<Modal>
			<div class="modal-privacy">
				<h3>privacy</h3>
				<div class="option">
					<input
						id="opt-dms"
						type="checkbox"
						checked={dms()}
						onInput={(e) => setDms(e.currentTarget.checked)}
						style="display: none;"
					/>
					<Checkbox checked={dms()} />
					<label for="opt-dms">
						<div>Allow direct messages</div>
						<div class="dim">
							Let others send direct messages to you. Bots, moderators, and
							friends can always start dms.
						</div>
					</label>
				</div>
				<div class="option">
					<input
						id="opt-rpc"
						type="checkbox"
						checked={rpc()}
						onInput={(e) => setRpc(e.currentTarget.checked)}
						style="display: none;"
					/>
					<Checkbox checked={rpc()} />
					<label for="opt-rpc">
						<div>Share rich presence</div>
						<div class="dim">
							Share rich presence with everyone else in this room. Friends can
							always view your rich presence.
						</div>
					</label>
				</div>
				<div class="option">
					<input
						id="opt-exif"
						type="checkbox"
						checked={exif()}
						onInput={(e) => setExif(e.currentTarget.checked)}
						style="display: none;"
					/>
					<Checkbox checked={exif()} />
					<label for="opt-exif">
						<div>Strip exif metadata</div>
						<div class="dim">
							Strip potentially sensitive exif metadata (ie. camera model or
							location) from images you upload.
						</div>
					</label>
				</div>
			</div>
		</Modal>
	);
};
