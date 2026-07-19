import { createSignal, Match, Switch } from "solid-js";
import { CheckboxOptionWithLabel } from "@/atoms/CheckboxOption";
import { useModals } from "@/contexts/modal";
import { autofocus } from "@/lib/autofocus";
import { Modal } from "./mod";

interface ModalRoomCreateOrJoinProps {
	onCreate: (data: { name: string; public: boolean } | null) => void;
	onInvite: (invite_code: string | null) => void;
}

export const ModalRoomCreateOrJoin = (props: ModalRoomCreateOrJoinProps) => {
	const [view, setView] = createSignal<"selection" | "create" | "invite">(
		"selection",
	);
	const [inviteCode, setInviteCode] = createSignal("");
	const [roomName, setRoomName] = createSignal("");
	const [isPublic, setIsPublic] = createSignal(false);
	const [, modalCtl] = useModals();

	return (
		<Modal class="room-create-or-join">
			<Switch>
				<Match when={view() === "selection"}>
					<h3>room</h3>
					<div class="bottom">
						<button
							type="button"
							class="primary"
							onClick={() => setView("create")}
						>
							create a room
						</button>
						<button
							type="button"
							class="primary"
							onClick={() => setView("invite")}
						>
							use invite
						</button>
					</div>
				</Match>

				<Match when={view() === "create"}>
					<h3>new room</h3>
					<form
						class="new-room"
						onSubmit={(e) => {
							e.preventDefault();
							if (!roomName().trim()) return;
							props.onCreate({
								name: roomName().trim(),
								public: isPublic(),
							});
							modalCtl.close();
						}}
					>
						<label class="room-name-input-label">
							<h3 class="dim">room name</h3>
							<input
								type="text"
								value={roomName()}
								onInput={(e) => setRoomName(e.currentTarget.value)}
								placeholder="my awesome room"
								required
								use:autofocus
							/>
						</label>

						<CheckboxOptionWithLabel
							id="room-public"
							checked={isPublic()}
							onChange={setIsPublic}
							seed="public"
							label="public room (visible to everyone)"
						/>

						<div class="bottom">
							<button
								type="button"
								class="button"
								onClick={() => setView("selection")}
							>
								Cancel
							</button>
							<button type="submit" class="button primary">
								Create Room
							</button>
						</div>
					</form>
				</Match>

				<Match when={view() === "invite"}>
					<h3>join room</h3>
					<form
						onSubmit={(e) => {
							e.preventDefault();
							props.onInvite(inviteCode());
							modalCtl.close();
						}}
					>
						<label>
							<h3 class="dim">invite code</h3>
							<input
								type="text"
								value={inviteCode()}
								onInput={(e) => setInviteCode(e.currentTarget.value)}
								placeholder="a1b2c3"
								required
							/>
						</label>
						<div class="bottom">
							<button
								type="button"
								class="button"
								onClick={() => setView("selection")}
							>
								Cancel
							</button>
							<button type="submit" class="button primary">
								Accept Invite
							</button>
						</div>
					</form>
				</Match>
			</Switch>
		</Modal>
	);
};
