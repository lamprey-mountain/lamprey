import { createSignal, Show, type VoidProps } from "solid-js";
import { createUpload, type User } from "sdk";
import { useCtx } from "../context.ts";
import { useApi } from "../api.tsx";
import { getThumbFromId, getUrl } from "../media/util.tsx";
import { Modal } from "../modal/mod.tsx";

export function Info(props: VoidProps<{ user: User }>) {
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

	const toggle = (setting: string) => () => {
		const c = ctx.userConfig();
		ctx.setUserConfig({
			...c,
			frontend: {
				...c.frontend,
				[setting]: c.frontend[setting] === "yes" ? "no" : "yes",
			},
		});
	};

	return (
		<div class="user-settings-info">
			<h2>info</h2>
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
			<br />
			<h3>appearance (todo: move to separate section)</h3>
			<br />
			<label>
				<input
					type="checkbox"
					checked={ctx.userConfig().frontend["message_pfps"] === "yes"}
					onInput={toggle("message_pfps")}
				/>{" "}
				show pfps in messages (experimental)
			</label>
			<br />
			<label>
				<input
					type="checkbox"
					checked={ctx.userConfig().frontend["underline_links"] === "yes"}
					onInput={toggle("underline_links")}
				/>{" "}
				always underline links
			</label>
			<br />
			<div class="danger">
				<h3>danger zone</h3>
				<label>
					<button
						onClick={() =>
							ctx.dispatch({
								do: "modal.open",
								modal: { type: "settings", user_id: props.user.id },
							})}
					>
						change password
					</button>
					<span style="margin-left:8px">change your password</span>
				</label>
				<br />
				<label>
					<button onClick={() => alert("todo")}>self destruct</button>
					<span style="margin-left:8px">this will delete your account</span>
				</label>
			</div>
		</div>
	);
}

export const ModalResetPassword = () => {
	const [password, setPassword] = createSignal("");
	const [confirmPassword, setConfirmPassword] = createSignal("");
	const ctx = useCtx();

	async function handlePasswordSet(e: SubmitEvent) {
		e.preventDefault();

		if (!password()) {
			ctx.dispatch({
				do: "modal.alert",
				text: "missing password",
			});
		}
		if (!confirmPassword()) {
			ctx.dispatch({
				do: "modal.alert",
				text: "missing confirmPassword",
			});
		}

		if (password() !== confirmPassword()) {
			ctx.dispatch({
				do: "modal.alert",
				text: "password !== confirmPassword",
			});
		}

		ctx.client.http.PUT("/api/v1/auth/password", {
			body: { password: password() },
		});
	}

	return (
		<Modal>
			<div class="auth">
				<section class="form-wrapper">
					reset password
					<form onSubmit={handlePasswordSet}>
						<label>
							<div class="label-text">password</div>
							<input
								class="input"
								type="password"
								placeholder="dolphins"
								value={password()}
								onInput={(e) => setPassword(e.currentTarget.value)}
							/>
						</label>
						<br />
						<label>
							<div class="label-text">confirm password</div>
							<input
								class="input"
								type="password"
								placeholder="dolphins"
								value={confirmPassword()}
								onInput={(e) => setConfirmPassword(e.currentTarget.value)}
							/>
						</label>
						<br />
						<br />
						<input class="submit-btn" type="submit" value={"set password"} />
					</form>
				</section>
			</div>
		</Modal>
	);
};
