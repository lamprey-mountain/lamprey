import { createEffect, createResource, createSignal, Show } from "solid-js";
import { Dropdown, MultiDropdown } from "../Dropdown";
import { Modal } from "./mod";
import { useApi } from "../api";
import { Time } from "sdk";

interface ModalInviteCreateProps {
	room_id?: string;
	channel_id?: string;
}

export const ModalInviteCreate = (props: ModalInviteCreateProps) => {
	const api = useApi();
	const [expiry, setExpiry] = createSignal<number | null>(null);
	const [maxUses, setMaxUses] = createSignal<number | null>(null);
	const [selectedRoleIds, setSelectedRoleIds] = createSignal<string[]>([]);
	const [inviteCode, setInviteCode] = createSignal<string>("");

	const roles = api.roles.list(() => props.room_id!);

	createEffect(() => {
		console.log("AAA", props.room_id, roles());
	});

	const handleCreate = async () => {
		const expires_at = expiry()
			? new Date(Date.now() + expiry()!).toISOString()
			: undefined;

		const body = {
			expires_at,
			max_uses: maxUses() ?? undefined,
			role_ids: selectedRoleIds().length > 0 ? selectedRoleIds() : undefined,
		};

		if (props.channel_id) {
			const { data, error } = await api.client.http.POST(
				"/api/v1/channel/{channel_id}/invite",
				{
					params: { path: { channel_id: props.channel_id } },
					body,
				},
			);
			if (data) setInviteCode(data.code);
			if (error) console.error(error);
		} else if (props.room_id) {
			const { data, error } = await api.client.http.POST(
				"/api/v1/room/{room_id}/invite",
				{
					params: { path: { room_id: props.room_id } },
					body,
				},
			);
			if (data) setInviteCode(data.code);
			if (error) console.error(error);
		}
	};

	const inviteLink = () =>
		inviteCode() && `${window.location.origin}/invite/${inviteCode()}`;

	const copyToClipboard = () => {
		navigator.clipboard.writeText(inviteLink());
	};

	return (
		<Modal>
			<div class="modal-invite-create">
				<h2>create invite</h2>
				<div style="margin-top:8px">
					<h3 class="dim">expire after</h3>
					<Dropdown
						selected={expiry()}
						onSelect={(v) => setExpiry(v)}
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
						selected={maxUses()}
						onSelect={(v) => setMaxUses(v)}
						options={[
							{ item: null, label: "no limit" },
							{ item: 1, label: "1 use" },
							{ item: 5, label: "5 uses" },
							{ item: 10, label: "10 uses" },
							{ item: 100, label: "100 uses" },
						]}
					/>
				</div>
				<Show when={props.room_id && (roles()?.items?.length ?? 0) > 0}>
					<div style="margin-top:8px">
						<h3 class="dim">grant roles</h3>
						<MultiDropdown
							selected={selectedRoleIds()}
							onSelect={(id) => setSelectedRoleIds([...selectedRoleIds(), id])}
							onRemove={(id) =>
								setSelectedRoleIds(selectedRoleIds().filter((i) => i !== id))}
							options={roles()?.items
								?.filter((r) => r.id !== props.room_id)
								.map((r) => ({
									item: r.id,
									label: r.name,
								})) ?? []}
							placeholder="select roles..."
							style="width:min-content"
						/>
					</div>
				</Show>
				<div class="invite-create-input-wrapper">
					<input
						type="text"
						readOnly
						placeholder="a1b2c3"
						value={inviteLink()}
						onClick={(e) => {
							e.currentTarget.select();
							navigator.clipboard.writeText(inviteLink());
						}}
					/>
					<button class="primary" onClick={copyToClipboard}>
						{inviteCode() ? "copy" : "create"}
					</button>
				</div>
			</div>
		</Modal>
	);
};
