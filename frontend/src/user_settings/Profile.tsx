import { createSignal, Show, type VoidProps } from "solid-js";
import { createUpload, type User } from "sdk";
import { useApi } from "../api";
import { useCtx } from "../context";
import { getThumbFromId } from "../media/util";
import { Copyable } from "../util";
import { useModals } from "../contexts/modal";

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

	const setAvatar = async (f: File) => {
		if (f) {
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
		} else {
			modalCtl.confirm("remove avatar?", (conf) => {
				if (!conf) return;
				setEditingAvatar(null);
			});
		}
	};

	let avatarInputEl!: HTMLInputElement;

	const openAvatarPicker = () => {
		avatarInputEl?.click();
	};

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
				<Show
					when={editingAvatar()}
					fallback={
						<div
							onClick={openAvatarPicker}
							class="avatar"
						>
						</div>
					}
				>
					<img
						onClick={openAvatarPicker}
						src={getThumbFromId(editingAvatar()!, 64)}
						class="avatar"
					/>
				</Show>
			</div>
			<div>
				user id: <Copyable>{props.user.id}</Copyable>
			</div>
			{isDirty() && (
				<div class="savebar">
					<div class="inner">
						<div class="warning">you have unsaved changes</div>
						<button class="reset" onClick={reset}>
							cancel
						</button>
						<button class="save" onClick={save}>
							save
						</button>
					</div>
				</div>
			)}
			<input
				style="display:none"
				ref={avatarInputEl}
				type="file"
				onInput={(e) => {
					const f = e.target.files?.[0];
					if (f) setAvatar(f);
				}}
			/>
		</div>
	);
}
