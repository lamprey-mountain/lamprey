import { createSignal } from "solid-js";
import { Modal } from "./mod";
import { useModals } from "../contexts/modal";
import type { Api } from "@/api";
import { DurationInput } from "../atoms/DurationInput";

interface ModalBanProps {
	api: Api;
	room_id: string;
	user_id?: string;
}

const banReasons = [
	"dude, chill",
	"they hurt my feelings",
	"they broke bad",
	"they were the impostor among us",
	"goofy ahh telegram scam",
];

export const ModalBan = (props: ModalBanProps) => {
	const [, modalCtl] = useModals();
	const user = () => props.api.users.cache.get(props.user_id!);
	const room_member = () =>
		props.api.room_members.cache.get(`${props.room_id}:${props.user_id}`);
	const [reason, setReason] = createSignal("");
	const [duration, setDuration] = createSignal<number | "forever" | null>(
		"forever",
	);
	const [loading, setLoading] = createSignal(false);

	const handleBan = async () => {
		if (!props.user_id) return;

		setLoading(true);
		try {
			const body: { reason?: string; expires_at?: string | null } = {
				reason: reason() || undefined,
			};

			if (duration() !== "forever" && duration() !== null) {
				const durationMs = (duration() as number) * 1000;
				body.expires_at = new Date(Date.now() + durationMs).toISOString();
			}

			await props.api.client.http.PUT(
				"/api/v1/room/{room_id}/ban/{user_id}",
				{
					params: {
						path: {
							room_id: props.room_id,
							user_id: props.user_id,
						},
					},
					body,
				},
			);
			modalCtl.close();
		} catch (err) {
			console.error("Failed to ban user:", err);
			modalCtl.alert("Failed to ban user");
		} finally {
			setLoading(false);
		}
	};

	const displayName = () =>
		room_member()?.override_name ?? user()?.name ?? props.user_id;

	const placeholderReason =
		banReasons[Math.floor(Math.random() * banReasons.length)];

	return (
		<Modal>
			<div class="modal-ban">
				<h3>
					ban <strong>{displayName()}</strong>
				</h3>
				<p>
					Are you sure you want to ban{" "}
					<strong>{displayName()}</strong>? They will not be able to rejoin the
					room.
				</p>

				<div style="margin-top: 16px;">
					<label for="ban-duration">duration</label>
					<DurationInput
						value={duration() ?? undefined}
						onInput={(d) => setDuration(d)}
						showForever
					/>
				</div>

				<div style="margin-top: 16px;">
					<label for="ban-reason">reason (optional)</label>
					<input
						id="ban-reason"
						type="text"
						value={reason()}
						onInput={(e) => setReason(e.currentTarget.value)}
						placeholder={placeholderReason}
						style="width: 100%; margin-top: 4px;"
					/>
				</div>

				<div class="bottom">
					<button
						class="danger"
						onClick={handleBan}
						disabled={loading()}
					>
						{loading() ? "banning..." : "ban"}
					</button>
					<button onClick={() => modalCtl.close()} disabled={loading()}>
						cancel
					</button>
				</div>
			</div>
		</Modal>
	);
};
