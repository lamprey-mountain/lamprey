import { createSignal } from "solid-js";
import { Dropdown } from "../Dropdown";
import { Checkbox } from "../icons";
import { Modal } from "./mod";
import { CheckboxOption } from "../atoms/CheckboxOption";

interface ModalNotificationsProps {
	room_id: string;
}

export const ModalNotifications = (props: ModalNotificationsProps) => {
	const [everyone, setEveryone] = createSignal(true);
	const [role, setRole] = createSignal(true);
	const [messages, setMessages] = createSignal<
		"Everything" | "Watching" | "Mentions" | "Nothing"
	>("Mentions");
	const [threads, setThreads] = createSignal<"Notify" | "Inbox" | "Nothing">(
		"Inbox",
	);

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
							{ item: "3h", label: "3 hours" },
							{ item: "8h", label: "8 hours" },
							{ item: "1d", label: "1 day" },
							{ item: "1w", label: "1 week" },
							{ item: "forever", label: "forever" },
						]}
					/>
				</div>
				<div>
					<h3 class="dim">messages</h3>
					<Dropdown
						selected={messages()}
						onSelect={(value) => value && setMessages(value as any)}
						options={[
							// You will be notified for all messages.
							{ item: "Everything", label: "everything" },

							// You will be notified for mentions only; messages go to inbox.
							{ item: "Watching", label: "watching" },

							// You will be notified for mentions only.
							{ item: "Mentions", label: "mentions" },

							// You won't be notified for anything.
							{ item: "Nothing", label: "nothing" },
						]}
					/>
				</div>
				<div>
					<h3 class="dim">threads</h3>
					<Dropdown
						selected={threads()}
						onSelect={(value) => value && setThreads(value as any)}
						options={[
							// You will be notified whenever a new thread is created.
							{ item: "Notify", label: "notify" },

							// All new threads will be added to your inbox.
							{ item: "Inbox", label: "inbox" },

							// Ignore new threads.
							{ item: "Nothing", label: "nothing" },
						]}
					/>
				</div>
				<div>
					<h3 class="dim">special notifications</h3>
					<CheckboxOption
						id="opt-everyone"
						checked={everyone()}
						onChange={setEveryone}
						seed={`modal-notifications-${props.room_id}-everyone`}
					>
						<Checkbox
							checked={everyone()}
							seed={`modal-notifications-${props.room_id}-everyone`}
						/>
						<label for="opt-everyone">
							<div>Enable @everyone and @here</div>
							<div class="dim">
								You will receive notifications when @everyone or @here is
								mentioned.
							</div>
						</label>
					</CheckboxOption>
					<CheckboxOption
						id="opt-role"
						checked={role()}
						onChange={setRole}
						seed={`modal-notifications-${props.room_id}-role`}
					>
						<Checkbox
							checked={role()}
							seed={`modal-notifications-${props.room_id}-role`}
						/>
						<label for="opt-role">
							<div>Enable all role mentions</div>
							<div class="dim">
								You will receive notifications when any @role you have is
								mentioned.
							</div>
						</label>
					</CheckboxOption>
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
									{ item: "Everything", label: "everything" },

									// You will be notified for mentions only.
									{ item: "Watching", label: "watching" },

									// You will be notified for mentions only.
									{ item: "Mentions", label: "mentions" },

									// You won't be notified for anything.
									{ item: "Nothing", label: "nothing" },
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
