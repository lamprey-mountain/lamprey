import { Show, type VoidProps } from "solid-js";
import { createUpload, type User } from "sdk";
import { useApi } from "../api";
import { useCtx } from "../context";
import { getThumbFromId } from "../media/util";

// TODO: allow uploading banner

export function Profile(props: VoidProps<{ user: User }>) {
	const api = useApi();
	const ctx = useCtx();

	const setName = () => {
		ctx.dispatch({
			do: "modal.prompt",
			text: "name?",
			cont(name) {
				if (!name) return;
				ctx.client.http.PATCH("/api/v1/user/{user_id}", {
					params: { path: { user_id: "@self" } },
					body: { name },
				});
			},
		});
	};

	const setDescription = () => {
		ctx.dispatch({
			do: "modal.prompt",
			text: "description?",
			cont(description) {
				if (typeof description !== "string") return;
				ctx.client.http.PATCH("/api/v1/user/{user_id}", {
					params: { path: { user_id: "@self" } },
					body: { description },
				});
			},
		});
	};

	const setAvatar = async (f: File) => {
		if (f) {
			await createUpload({
				client: api.client,
				file: f,
				onComplete(media) {
					api.client.http.PATCH("/api/v1/user/{user_id}", {
						params: { path: { user_id: "@self" } },
						body: { avatar: media.id },
					});
				},
				onFail(_error) {},
				onPause() {},
				onResume() {},
				onProgress(_progress) {},
			});
		} else {
			ctx.dispatch({
				do: "modal.confirm",
				text: "remove avatar?",
				cont(conf) {
					if (!conf) return;
					ctx.client.http.PATCH("/api/v1/user/{user_id}", {
						params: { path: { user_id: "@self" } },
						body: { avatar: null },
					});
				},
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
				<div
					class="name"
					onClick={setName}
				>
					{props.user.name}
				</div>
				<div class="description" onClick={setDescription}>
					<Show
						when={props.user.description}
						fallback={<em style="color:#aaa">click to add description</em>}
					>
						{props.user.description}
					</Show>
				</div>
				<Show
					when={props.user.avatar}
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
						src={getThumbFromId(props.user.avatar!, 64)}
						class="avatar"
					/>
				</Show>
			</div>
			<div>
				id: <code class="select-all">{props.user.id}</code>
			</div>
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
