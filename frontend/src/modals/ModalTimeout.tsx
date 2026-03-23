import { createSignal } from "solid-js";
import { Modal } from "./mod";
import { useModals } from "../contexts/modal";
import type { Api } from "@/api";
import { DurationInput } from "../atoms/DurationInput";

interface ModalTimeoutProps {
	api: Api;
	room_id: string;
	user_id: string;
}

const timeoutReasons = [
	"dude, chill",
	"they hurt my feelings",
	"they broke bad",
	"they were the impostor among us",
	"goofy ahh telegram scam",
];

export const ModalTimeout = (props: ModalTimeoutProps) => {
	const [, modalCtl] = useModals();
	const user = props.api.users.fetch(() => props.user_id);
	const room_member = props.api.room_members.fetch(
		() => props.room_id,
		() => props.user_id,
	);
	const [duration, setDuration] = createSignal<number | null>(3600);
	const [reason, setReason] = createSignal("");
	const [loading, setLoading] = createSignal(false);

	const handleTimeout = async () => {
		if (!duration()) return;
		setLoading(true);
		try {
			const durationMs = (duration() as number) * 1000;
			await props.api.client.http.PATCH(
				"/api/v1/room/{room_id}/member/{user_id}",
				{
					params: {
						path: {
							room_id: props.room_id,
							user_id: props.user_id,
						},
					},
					body: {
						timeout_until: new Date(Date.now() + durationMs).toISOString(),
						reason: reason() || undefined,
					},
				},
			);
			modalCtl.close();
		} catch (err) {
			console.error("Failed to timeout user:", err);
			modalCtl.alert("Failed to timeout user");
		} finally {
			setLoading(false);
		}
	};

	const displayName = () =>
		room_member()?.override_name ?? user()?.name ?? props.user_id;

	const placeholderReason =
		timeoutReasons[Math.floor(Math.random() * timeoutReasons.length)];

	return (
		<Modal>
			<div class="modal-timeout">
				<h3>
					timeout <strong>{displayName()}</strong>
				</h3>
				<p>
					Timeout <strong>{displayName()}</strong> for:
				</p>

				<div style="margin: 16px 0;">
					<DurationInput
						value={duration() ?? undefined}
						onInput={(d) => setDuration(typeof d === "number" ? d : null)}
					/>
				</div>

				<div>
					<label for="timeout-reason">reason (optional)</label>
					<input
						id="timeout-reason"
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
						onClick={handleTimeout}
						disabled={loading() || !duration()}
					>
						{loading() ? "timing out..." : "timeout"}
					</button>
					<button onClick={() => modalCtl.close()} disabled={loading()}>
						cancel
					</button>
				</div>
			</div>
		</Modal>
	);
};
