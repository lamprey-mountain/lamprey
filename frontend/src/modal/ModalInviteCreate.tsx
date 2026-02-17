import { Dropdown } from "../Dropdown";
import { Modal } from "./mod";

interface ModalInviteCreateProps {
	room_id?: string;
	channel_id?: string;
}

export const ModalInviteCreate = (props: ModalInviteCreateProps) => {
	return (
		<Modal>
			<div class="modal-invite-create">
				<h2>create invite</h2>
				<div style="margin-top:8px">
					<h3 class="dim">expire after</h3>
					<Dropdown
						options={[
							{ item: null, label: "never" },
							{ item: 1000 * 60 * 5, label: "5 minutes" },
							{ item: 1000 * 60 * 60, label: "1 hour" },
							{ item: 1000 * 60 * 60 * 6, label: "6 hours" },
							{ item: 1000 * 60 * 60 * 24, label: "1 day" },
							{ item: 1000 * 60 * 60 * 24 * 7, label: "1 week" },
						]}
					/>
				</div>
				<div style="margin-top:8px">
					<h3 class="dim">use count</h3>
					<Dropdown
						options={[
							{ item: null, label: "no limit" },
							{ item: 1, label: "1 use" },
							{ item: 5, label: "5 uses" },
							{ item: 10, label: "10 uses" },
							{ item: 100, label: "100 uses" },
						]}
					/>
				</div>
				{/* TODO: selecting roles */}
				<div style="margin-top:8px;display:flex;max-width:100%">
					<input type="text" placeholder="click to copy" />
					<button class="primary">create</button>
				</div>
			</div>
		</Modal>
	);
};
