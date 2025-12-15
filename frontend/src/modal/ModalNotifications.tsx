import { createSignal } from "solid-js";
import { Dropdown } from "../Dropdown";
import { Checkbox } from "../icons";
import { Modal } from "./mod";

interface ModalNotificationsProps {
	room_id: string;
}

export const ModalNotifications = (props: ModalNotificationsProps) => {
	const [everyone, setEveryone] = createSignal(true);
	const [role, setRole] = createSignal(true);
	const [thread, setThread] = createSignal(true);

	return (
		<Modal>
			<div class="modal-notifications">
				<h3>notifications</h3>
				<div>
					<h3 class="dim">mute (room_name) for</h3>
					<Dropdown
						options={[
							{ item: "none", label: "unmute" },
							{ item: "15m", label: "15 minutes" },
							{ item: "3h", label: "15 minutes" },
							{ item: "8h", label: "8 hours" },
							{ item: "1d", label: "1 day" },
							{ item: "1w", label: "1 week" },
							{ item: "forever", label: "forever" },
						]}
					/>
				</div>
				<div>
					<h3 class="dim">default notifications</h3>
					<Dropdown
						options={[
							// Uses your default notification setting.
							{ item: "default", label: "default" },

							// You will be notified for all messages.
							{ item: "everything", label: "everything" },

							// You will be notified for mentions only.
							{ item: "mentions", label: "mentions" },

							// You won't be notified for anything.
							{ item: "nothing", label: "nothing" },
						]}
					/>
				</div>
				<div>
					<h3 class="dim">special notifications</h3>
					<div class="option">
						<input
							id="opt-everyone"
							type="checkbox"
							checked={everyone()}
							onInput={(e) => setEveryone(e.currentTarget.checked)}
							style="display: none;"
						/>
						<Checkbox checked={everyone()} />
						<label for="opt-everyone">
							<div>Enable @everyone and @here</div>
							<div class="dim">
								You will receive notifications when @everyone or @here is
								mentioned.
							</div>
						</label>
					</div>
					<div class="option">
						<input
							id="opt-role"
							type="checkbox"
							checked={role()}
							onInput={(e) => setRole(e.currentTarget.checked)}
							style="display: none;"
						/>
						<Checkbox checked={role()} />
						<label for="opt-role">
							<div>Enable all role mentions</div>
							<div class="dim">
								You will receive notifications when any @role you have is
								mentioned.
							</div>
						</label>
					</div>
					<div class="option">
						<input
							id="opt-thread"
							type="checkbox"
							checked={thread()}
							onInput={(e) => setThread(e.currentTarget.checked)}
							style="display: none;"
						/>
						<Checkbox checked={thread()} />
						<label for="opt-thread">
							<div>New threads</div>
							<div class="dim">
								You will receive notifications when a new thread is created.
							</div>
						</label>
					</div>
					{/* TODO: (when impl'd) mobile push notifications */}
					{/* TODO: (when impl'd) calendar event */}
				</div>
				<div>
					<h3 class="dim">channel settings</h3>
					<div style="display:flex;background:#aaa2;align-items:center;justify-content:space-between;padding:0 4px">
						<div>#channel</div>
						<div style="display:flex;align-items:center;">
							<Dropdown
								options={[
									// Uses your default notification setting.
									{ item: "default", label: "default" },

									// You will be notified for all messages.
									{ item: "everything", label: "everything" },

									// You will be notified for mentions only.
									{ item: "mentions", label: "mentions" },

									// You won't be notified for anything.
									{ item: "nothing", label: "nothing" },
								]}
							/>
							{/* TODO: show menu to mute this channel */}
							<button style="background:#111;border:solid #222 1px; padding:4px;margin-left:4px">
								mute
							</button>
							{/* TODO: show option to deleet this setting */}
						</div>
					</div>
				</div>
			</div>
		</Modal>
	);
};
