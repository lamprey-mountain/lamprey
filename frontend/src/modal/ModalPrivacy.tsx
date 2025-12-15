import { createSignal } from "solid-js";
import { Modal } from "./mod";
import { Checkbox } from "../icons";
import { Dropdown } from "../Dropdown";

interface ModalPrivacyProps {
	room_id: string;
}

export const ModalPrivacy = (props: ModalPrivacyProps) => {
	const [dms, setDms] = createSignal(false);
	const [rpc, setRpc] = createSignal(false);
	const [exif, setExif] = createSignal<"none" | "location" | "all">("location");

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
							Let others send direct messages to you. Bots, moderators, and friends can always start dms.
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
							Share rich presence with everyone else in this room. Friends can always view your rich presence.
						</div>
					</label>
				</div>
				<div class="option" style="z-index:999;flex-direction:column;align-items:start">
					<h3 class="dim">EXIF Metadata</h3>
					<Dropdown
						options={[
							{ item: "all", label: "Strip all data" },
							{ item: "location", label: "Strip location data" },
							{ item: "none", label: "Don't strip anything" },
						]}
						onSelect={(it) => setExif(it)}
						required
						selected={exif()}
					/>
				</div>
			</div>
		</Modal>
	);
};
