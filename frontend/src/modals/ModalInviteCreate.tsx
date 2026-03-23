import { useCurrentUser } from "../contexts/currentUser.tsx";
import {
	createEffect,
	createMemo,
	createResource,
	createSignal,
	Show,
} from "solid-js";
import { Dropdown, MultiDropdown } from "../atoms/Dropdown";
import { Modal } from "./mod";
import { useApi, useChannels2, useRooms2 } from "../api";
import { Time } from "sdk";
import {
	calculatePermissions,
	type PermissionContext,
} from "../permission-calculator";

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
	const [creating, setCreating] = createSignal(false);
	const currentUser = useCurrentUser();

	const api2 = useRooms2();
	const roles = api.roles.list(() => props.room_id as string);

	const currentUserId = () => currentUser()?.id;

	const canApplyRoles = createMemo(() => {
		const roomId = props.room_id;
		const userId = currentUserId();
		if (!roomId || !userId) return false;
		const permissionContext: PermissionContext = {
			api,
			channels: useChannels2(),
			rooms: api2,
			room_id: roomId,
			channel_id: props.channel_id,
		};
		const { permissions, rank } = calculatePermissions(
			permissionContext,
			userId,
		);
		const hasRoleApply = permissions.has("RoleApply") ||
			permissions.has("Admin");
		const room = api2.use(() => roomId)();
		const isOwner = room?.owner_id === userId;
		return { canApply: hasRoleApply, rank, isOwner };
	});

	const availableRoles = createMemo(() => {
		const roleItems = roles()?.items;
		const roomId = props.room_id;
		if (!roleItems || !roomId) return [];
		const { canApply, rank, isOwner } = canApplyRoles() as any;
		if (!canApply) return [];
		return roleItems
			.filter((r) => r.id !== roomId)
			.filter((r) => isOwner || rank > r.position)
			.map((r) => ({
				item: r.id,
				label: r.name,
			}));
	});

	const handleCreate = async () => {
		setCreating(true);
		const exp = expiry();
		const expires_at = exp
			? new Date(Date.now() + exp).toISOString()
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
			if (data) {
				setInviteCode(data.code);
				queueMicrotask(() => inputRef()?.select());
			}
			if (error) console.error(error);
		} else if (props.room_id) {
			const { data, error } = await api.client.http.POST(
				"/api/v1/room/{room_id}/invite",
				{
					params: { path: { room_id: props.room_id } },
					body,
				},
			);
			if (data) {
				setInviteCode(data.code);
				queueMicrotask(() => inputRef()?.select());
			}
			if (error) console.error(error);
		}
		setCreating(false);
	};

	const inviteLink = () =>
		inviteCode() && `${window.location.origin}/invite/${inviteCode()}`;

	const copyToClipboard = () => {
		navigator.clipboard.writeText(inviteLink());
	};

	const [inputRef, setInputRef] = createSignal<HTMLInputElement>();

	const selectAndCopy = () => {
		const input = inputRef();
		if (input) {
			input.select();
			navigator.clipboard.writeText(inviteLink());
		}
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
						placeholder="never"
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
						placeholder="no limit"
					/>
				</div>
				<Show when={availableRoles().length > 0}>
					<div style="margin-top:8px">
						<h3 class="dim">grant roles</h3>
						<MultiDropdown
							selected={selectedRoleIds()}
							onSelect={(id) => setSelectedRoleIds([...selectedRoleIds(), id])}
							onRemove={(id) =>
								setSelectedRoleIds(selectedRoleIds().filter((i) => i !== id))}
							options={availableRoles()}
							placeholder="select roles..."
							style="width:min-content"
						/>
					</div>
				</Show>
				<div class="invite-create-input-wrapper">
					<input
						ref={setInputRef}
						type="text"
						readOnly
						placeholder={creating() ? "creating..." : "a1b2c3"}
						value={inviteLink()}
						onClick={(e) => {
							e.currentTarget.select();
							navigator.clipboard.writeText(inviteLink());
						}}
					/>
					<button
						class="primary"
						onClick={() => {
							if (inviteCode()) {
								selectAndCopy();
							} else {
								handleCreate();
							}
						}}
					>
						{inviteCode() ? "copy" : "create"}
					</button>
				</div>
			</div>
		</Modal>
	);
};
