import { createSignal, Show, type VoidProps } from "solid-js";
import { createUpload, type User } from "sdk";
import { useApi } from "../api";
import { useCtx } from "../context";
import { Copyable } from "../util";
import { useModals } from "../contexts/modal";
import { Avatar } from "../User.tsx";
import { Savebar } from "../atoms/Savebar";

// TODO(#753): allow uploading banner

export function Profile(props: VoidProps<{ user: User }>) {
	const api = useApi();
	const ctx = useCtx();
	const [, modalCtl] = useModals();

	const [editingName, setEditingName] = createSignal(props.user.name);
	const [editingDescription, setEditingDescription] = createSignal(
		props.user.description,
	);
	const [editingAvatar, setEditingAvatar] = createSignal(props.user.avatar);

	const isDirty = () =>
		editingName() !== props.user.name ||
		editingDescription() !== props.user.description ||
		editingAvatar() !== props.user.avatar;

	const save = () => {
		ctx.client.http.PATCH("/api/v1/user/{user_id}", {
			params: { path: { user_id: "@self" } },
			body: {
				name: editingName(),
				description: editingDescription(),
				avatar: editingAvatar(),
			},
		});
	};

	const reset = () => {
		setEditingName(props.user.name);
		setEditingDescription(props.user.description);
		setEditingAvatar(props.user.avatar);
	};

	const setAvatarFile = async (f: File) => {
		await createUpload({
			client: api.client,
			file: f,
			onComplete(media) {
				setEditingAvatar(media.id);
			},
			onFail(_error) {},
			onPause() {},
			onResume() {},
			onProgress(_progress) {},
		});
	};

	const removeAvatar = async () => {
		setEditingAvatar(null);
	};

	let avatarInputEl!: HTMLInputElement;

	const openAvatarPicker = () => {
		avatarInputEl?.click();
	};

	const userWithAvatar = () => ({
		id: props.user.id,
		name: props.user.name,
		avatar: editingAvatar(),
		banner: null,
		description: null,
		flags: 0,
		presence: { status: "Offline" as const, activities: [] },
		relationship: null,
		user_config: null,
	});

	return (
		<div class="user-settings-info">
			<h2>profile</h2>
			<div class="box profile">
				<input
					class="name"
					type="text"
					value={editingName()}
					onInput={(e) => setEditingName(e.target.value)}
				/>
				<textarea
					class="description"
					value={editingDescription()}
					onInput={(e) => setEditingDescription(e.target.value)}
				/>
				<div class="avatar-uploader" onClick={openAvatarPicker}>
					<div class="avatar-inner">
						<Avatar user={userWithAvatar()} />
						<div class="overlay">upload avatar</div>
					</div>
					<Show when={editingAvatar()}>
						<button
							class="remove"
							onClick={(e) => {
								e.stopPropagation();
								removeAvatar();
							}}
						>
							remove
						</button>
					</Show>
					<input
						style="display:none"
						ref={avatarInputEl}
						type="file"
						onInput={(e) => {
							const f = e.target.files?.[0];
							if (f) setAvatarFile(f);
						}}
					/>
				</div>
			</div>
			<div>
				user id: <Copyable>{props.user.id}</Copyable>
			</div>
			<Savebar
				show={isDirty()}
				onCancel={reset}
				onSave={save}
			/>
		</div>
	);
}
