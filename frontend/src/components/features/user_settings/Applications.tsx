import { useCurrentUser } from "../../../contexts/currentUser.tsx";
import type { RoomT } from "../../../types";
import {
	createEffect,
	createResource,
	createSignal,
	For,
	onCleanup,
	Show,
	type VoidProps,
} from "solid-js";
import { type Application, createUpload, type Room, type User } from "sdk";
import { useApi, useRooms2 } from "../../../api.tsx";
import { Copyable } from "../../../utils/general";
import { createStore, reconcile } from "solid-js/store";
import { useCtx } from "../../../context.ts";
import { useFloating } from "solid-floating-ui";
import { ReferenceElement, shift } from "@floating-ui/dom";
import { usePermissions } from "../../../hooks/usePermissions.ts";
import { useModals } from "../../../contexts/modal";
import { Checkbox } from "../../../icons";
import { Resizable } from "../../../atoms/Resizable";
import { CheckboxOption } from "../../../atoms/CheckboxOption";
import { getThumbFromId } from "../../../media/util";
import { Avatar } from "../../../User.tsx";
import { Savebar } from "../../../atoms/Savebar";
import fuzzysort from "fuzzysort";

// TODO: in create session and rotate oauth token, make the secret Copyable

const SessionList = (props: { appId: string }) => {
	const api = useApi();
	const [, modalctl] = useModals();

	const [sessions, { refetch }] = createResource(async () => {
		const { data } = await api.client.http.GET("/api/v1/session", {
			headers: { "x-puppet-id": props.appId },
		});
		return data?.items ?? [];
	});

	const revokeSession = (sessionId: string) => {
		modalctl.confirm(
			"Are you sure you want to revoke this session?",
			async (confirmed) => {
				if (confirmed) {
					await api.client.http.DELETE("/api/v1/session/{session_id}", {
						params: { path: { session_id: sessionId } },
					});
					refetch();
				}
			},
		);
	};

	const renameSession = (sessionId: string) => {
		modalctl.prompt("New session name?", async (name) => {
			if (name === null) return;
			await api.client.http.PATCH("/api/v1/session/{session_id}", {
				params: { path: { session_id: sessionId } },
				body: { name: name || null },
			});
			refetch();
		});
	};

	return (
		<div class="sessions-list">
			<h4>Sessions</h4>
			<Show when={sessions.loading}>Loading sessions...</Show>
			<Show when={sessions() && sessions()!.length > 0}>
				<ul>
					<For each={sessions()}>
						{(session) => (
							<li>
								<div style="display:flex">
									<div style="flex:1">{session.name || session.id}</div>
									<button onClick={() => renameSession(session.id)}>
										Rename
									</button>
									<button
										class="danger"
										onClick={() => revokeSession(session.id)}
									>
										Revoke
									</button>
								</div>
							</li>
						)}
					</For>
				</ul>
			</Show>
			<Show when={sessions() && sessions()!.length === 0}>
				<p>No active sessions.</p>
			</Show>
		</div>
	);
};

type AppEditState = ReturnType<typeof useAppEditor>;

function useAppEditor(initial: Application | null) {
	const [app, setApp] = createStore(
		initial ?? { id: null } as unknown as Application,
	);
	const [name, setName] = createSignal(initial?.name ?? "");
	const [desc, setDesc] = createSignal(initial?.description ?? undefined);
	const [avatar, setAvatar] = createSignal<string | null>(
		initial?.avatar ?? null,
	);

	return { app, setApp, name, setName, desc, setDesc, avatar, setAvatar };
}

export function Applications(_props: VoidProps<{ user: User }>) {
	const api = useApi();
	const [, modalctl] = useModals();

	async function create() {
		modalctl.prompt("New app name?", async (name) => {
			if (!name) return;
			await api.client.http.POST("/api/v1/app", {
				body: {
					name,
					bridge: null,
					public: false,
				},
			});
			refetch();
		});
	}

	const [list, { refetch }] = createResource(async () => {
		const { data } = await api.client.http.GET("/api/v1/app", {
			params: { query: { limit: 100 } },
		});
		return data;
	});

	const [apps, setApps] = createStore<Application[]>([]);
	const [originalApps, setOriginalApps] = createSignal<Application[]>([]);

	createEffect(() => {
		if (list()) {
			setOriginalApps(JSON.parse(JSON.stringify(list()!.items)));
			setApps(reconcile(list()!.items));
		}
	});

	const hasUnsavedChanges = () => {
		const orig = originalApps();
		if (orig.length === 0 && apps.length === 0) return false;
		if (orig.length !== apps.length) return true;
		return JSON.stringify(orig) !== JSON.stringify(apps);
	};

	const cancelChanges = () => {
		setApps(reconcile(originalApps()));
	};

	const saveChanges = async () => {
		const promises = apps
			.map((app) => {
				const originalApp = originalApps().find((o) => o.id === app.id);
				if (
					originalApp && JSON.stringify(app) !== JSON.stringify(originalApp)
				) {
					return api.client.http.PATCH("/api/v1/app/{app_id}", {
						params: { path: { app_id: app.id } },
						body: {
							name: app.name,
							description: app.description,
							bridge: app.bridge,
							public: app.public,
							oauth_confidential: app.oauth_confidential,
							oauth_redirect_uris: app.oauth_redirect_uris,
						},
					});
				}
				return null;
			})
			.filter(Boolean);

		if (promises.length > 0) {
			await Promise.all(promises);
			setOriginalApps(JSON.parse(JSON.stringify(apps)));
			refetch();
		}
	};

	const [, modalCtl] = useModals();
	const rotateSecret = async (app_id: string) => {
		const { data } = await api.client.http.POST(
			"/api/v1/app/{app_id}/rotate-secret",
			{
				params: { path: { app_id } },
			},
		);
		modalCtl.alert(
			`your secret is ${data?.oauth_secret} (this can only be seen once)`,
		);
	};

	const [inviteApp, setInviteApp] = createSignal<
		{ app: Application; x: number; y: number }
	>();
	const InviteAppClear = () => setInviteApp();
	document.addEventListener("click", InviteAppClear);
	onCleanup(() => document.removeEventListener("click", InviteAppClear));

	const createSession = async (app_id: string) => {
		const { data } = await api.client.http.POST(
			"/api/v1/app/{app_id}/session",
			{
				params: { path: { app_id } },
				body: { name: "session" },
			},
		);
		modalCtl.alert(
			`your secret is ${data?.token} (this can only be seen once)`,
		);
	};

	const [search, setSearch] = createSignal("");

	const filteredApps = () => {
		const query = search();
		if (!query) return apps;
		const results = fuzzysort.go(query, apps, {
			key: "name",
			threshold: -10000,
		});
		return results.map((r) => r.obj);
	};

	const updateApp = (index: number, field: keyof Application, value: any) => {
		setApps(index, field, value);
	};

	const edit = useAppEditor(null);

	return (
		<div class="user-settings-applications">
			<div class="room-settings-roles">
				<div class="role-main">
					<h2>applications</h2>
					<header class="applications-header">
						<input
							type="search"
							placeholder="search"
							aria-label="search"
							onInput={(e) => setSearch(e.target.value)}
						/>
						<button type="button" class="primary big" onClick={create}>
							create
						</button>
					</header>
					<ul class="applications-list">
						<For each={filteredApps()}>
							{(app, index) => {
								const appWithAvatar = () => ({
									id: app.id,
									name: app.name,
									avatar: app.avatar ?? null,
									banner: null,
									description: null,
									bot: false,
									system: false,
									version_id: "",
									flags: 0,
									presence: { status: "Offline" as const, activities: [] },
									preferences: null,
								});

								return (
									<li
										onClick={() => {
											if (edit.app.id === app.id) {
												edit.setApp({ id: null } as unknown as Application);
											} else {
												edit.setApp(JSON.parse(JSON.stringify(app)));
												edit.setName(app.name);
												edit.setDesc(app.description || undefined);
												edit.setAvatar(app.avatar ?? null);
											}
										}}
									>
										<div class="info">
											<Avatar user={appWithAvatar()} pad={4} />
											<div style="display: flex; flex-direction:column;">
												<h3 class="name">{app.name}</h3>
												<Show when={app.description}>
													<div class="description">{app.description}</div>
												</Show>
											</div>
										</div>
									</li>
								);
							}}
						</For>
					</ul>
					<Savebar
						show={hasUnsavedChanges()}
						onCancel={cancelChanges}
						onSave={saveChanges}
					/>
				</div>
				<Show when={edit.app.id !== null}>
					<Resizable
						storageKey="app-editor-width"
						initialWidth={400}
						minWidth={300}
						maxWidth={800}
						classList={{ "role-edit-resizable": true }}
					>
						<AppEditor
							edit={edit}
							updateApp={updateApp}
							apps={apps}
							rotateSecret={rotateSecret}
							createSession={createSession}
							setInviteApp={setInviteApp}
							refetch={refetch}
						/>
					</Resizable>
				</Show>
			</div>
			<Show when={inviteApp()}>
				{(app) => (
					<InviteToRoom
						x={app().x}
						y={app().y}
						app={app().app}
					/>
				)}
			</Show>
		</div>
	);
}

const AppEditor = (
	props: {
		edit: AppEditState;
		updateApp: (index: number, field: keyof Application, value: any) => void;
		apps: Application[];
		rotateSecret: (app_id: string) => Promise<void>;
		createSession: (app_id: string) => Promise<void>;
		setInviteApp: (
			app: { app: Application; x: number; y: number } | undefined,
		) => void;
		refetch: () => void;
	},
) => {
	const api = useApi();
	const [, modalCtl] = useModals();
	const [activeTab, setActiveTab] = createSignal<
		"overview" | "oauth" | "sessions"
	>("overview");

	const appIndex = () =>
		props.apps.findIndex((a) => a.id === props.edit.app.id);

	const deleteApp = (app_id: string) => () => {
		modalCtl.confirm("are you sure?", (confirmed) => {
			if (!confirmed) return;
			api.client.http.DELETE("/api/v1/app/{app_id}", {
				params: { path: { app_id } },
			});
			props.edit.setApp({ id: null } as unknown as Application);
			props.refetch();
		});
	};

	const saveApp = () => {
		const index = appIndex();
		if (index === -1) return;

		const app = props.edit.app;
		const originalApp = props.apps[index];

		if (JSON.stringify(app) !== JSON.stringify(originalApp)) {
			api.client.http.PATCH("/api/v1/app/{app_id}", {
				params: { path: { app_id: app.id } },
				body: {
					name: props.edit.name(),
					description: props.edit.desc() ?? null,
					bridge: app.bridge,
					public: app.public,
					oauth_confidential: app.oauth_confidential,
					oauth_redirect_uris: app.oauth_redirect_uris,
					avatar: props.edit.avatar(),
				},
			}).then(() => {
				props.refetch();
			});
		}
	};

	const setAvatarFile = async (f: File) => {
		await createUpload({
			client: api.client,
			file: f,
			onComplete(media) {
				props.edit.setAvatar(media.id);
				props.edit.setApp("avatar", media.id);
			},
			onFail(_error) {},
			onPause() {},
			onResume() {},
			onProgress(_progress) {},
		});
	};

	const removeAvatar = async () => {
		props.edit.setAvatar(null);
		props.edit.setApp("avatar", null);
	};

	let avatarInputEl!: HTMLInputElement;

	const openAvatarPicker = () => {
		avatarInputEl?.click();
	};

	const appWithAvatar = () => ({
		id: props.edit.app.id,
		name: props.edit.name(),
		avatar: props.edit.avatar(),
		banner: null,
		description: null,
		bot: false,
		system: false,
		version_id: "",
		flags: 0,
		presence: { status: "Offline" as const, activities: [] },
		preferences: null,
	});

	const isDirty = () => {
		const index = appIndex();
		if (index === -1) return false;
		const originalApp = props.apps[index];
		return (
			JSON.stringify(props.edit.app) !== JSON.stringify(originalApp) ||
			props.edit.name() !== originalApp.name ||
			props.edit.desc() !== (originalApp.description ?? undefined) ||
			props.edit.avatar() !== (originalApp.avatar ?? null)
		);
	};

	return (
		<div class="role-edit appplication-edit">
			<div class="toolbar">
				<button
					onClick={() => {
						props.edit.setApp({ id: null } as unknown as Application);
					}}
				>
					close
				</button>
				<button
					disabled={!isDirty()}
					onClick={saveApp}
				>
					save
				</button>
				<button class="danger" onClick={deleteApp(props.edit.app.id!)}>
					delete app
				</button>
			</div>
			<div class="tabs">
				<button
					classList={{ active: activeTab() === "overview" }}
					onClick={() => setActiveTab("overview")}
				>
					overview
				</button>
				<button
					classList={{ active: activeTab() === "oauth" }}
					onClick={() => setActiveTab("oauth")}
				>
					oauth
				</button>
				<button
					classList={{ active: activeTab() === "sessions" }}
					onClick={() => setActiveTab("sessions")}
				>
					sessions
				</button>
			</div>
			<Show when={activeTab() === "overview"}>
				<div class="avatar-uploader" onClick={openAvatarPicker}>
					<div class="avatar-inner">
						<Avatar user={appWithAvatar()} />
						<div class="overlay">upload avatar</div>
					</div>
					<Show when={props.edit.avatar()}>
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
				<div>
					id <Copyable>{props.edit.app.id!}</Copyable>
				</div>
				<h3>name</h3>
				<input
					type="text"
					value={props.edit.name()}
					onInput={(e) => {
						props.edit.setApp("name", e.currentTarget.value);
						props.edit.setName(e.currentTarget.value);
					}}
				/>
				<div style="height: 8px" />
				<h3>description</h3>
				<textarea
					onInput={(e) => {
						props.edit.setApp("description", e.currentTarget.value || null);
						props.edit.setDesc(e.currentTarget.value || undefined);
					}}
				>
					{props.edit.desc() ?? ""}
				</textarea>
				<div style="height: 8px" />
				<CheckboxOption
					id={`app-${props.edit.app.id}-bridge`}
					checked={!!props.edit.app.bridge}
					onChange={(checked) => {
						props.edit.setApp("bridge", checked ? {} : null);
					}}
					seed={`app-${props.edit.app.id}-bridge`}
				>
					<Checkbox
						checked={!!props.edit.app.bridge}
						seed={`app-${props.edit.app.id}-bridge`}
					/>
					<label for={`app-${props.edit.app.id}-bridge`} style="display: block">
						<div>bridge</div>
						<div class="dim">can create puppets</div>
					</label>
				</CheckboxOption>
				<Show when={props.edit.app.bridge}>
					{(bridge) => (
						<div class="bridge-details" style="padding-left: 24px;">
							<h3>platform name</h3>
							<input
								type="text"
								value={bridge().platform_name ?? ""}
								onInput={(e) => {
									props.edit.setApp("bridge", {
										...bridge(),
										platform_name: e.currentTarget.value || null,
									});
								}}
							/>
							<div style="height: 8px" />
							<h3>platform url</h3>
							<input
								type="text"
								value={bridge().platform_url ?? ""}
								onInput={(e) => {
									props.edit.setApp("bridge", {
										...bridge(),
										platform_url: e.currentTarget.value || null,
									});
								}}
							/>
							<div style="height: 8px" />
							<h3>platform description</h3>
							<textarea
								onInput={(e) => {
									props.edit.setApp("bridge", {
										...bridge(),
										platform_description: e.currentTarget.value || null,
									});
								}}
							>
								{bridge().platform_description ?? ""}
							</textarea>
						</div>
					)}
				</Show>
				<CheckboxOption
					id={`app-${props.edit.app.id}-public`}
					checked={props.edit.app.public}
					onChange={(checked) => {
						props.edit.setApp("public", checked);
					}}
					seed={`app-${props.edit.app.id}-public`}
				>
					<Checkbox
						checked={props.edit.app.public}
						seed={`app-${props.edit.app.id}-public`}
					/>
					<label for={`app-${props.edit.app.id}-public`} style="display: block">
						<div>public</div>
						<div class="dim">anyone can add and use this bot</div>
					</label>
				</CheckboxOption>
				<button
					style="margin-left:4px"
					onClick={(e) => {
						e.stopImmediatePropagation();
						props.setInviteApp({
							app: props.edit.app,
							x: e.clientX,
							y: e.clientY,
						});
					}}
				>
					add to room
				</button>
			</Show>
			<Show when={activeTab() === "oauth"}>
				<div class="oauth">
					<label class="option">
						<input
							type="checkbox"
							checked={props.edit.app.oauth_confidential}
							onInput={(e) =>
								props.edit.setApp(
									"oauth_confidential",
									e.currentTarget.checked,
								)}
							style="display: none;"
						/>
						<Checkbox
							checked={props.edit.app.oauth_confidential}
							seed={`app-${props.edit.app.id}-oauth-confidential`}
						/>
						<div>
							<div>confidential</div>
							<div class="dim">can keep secrets</div>
						</div>
					</label>
					<h3 class="dim">redirect uris</h3>
					<ul>
						<For each={props.edit.app.oauth_redirect_uris}>
							{(uri, uriIndex) => (
								<li>
									<input
										type="text"
										value={uri}
										onInput={(e) => {
											const newUris = [
												...props.edit.app.oauth_redirect_uris ?? [],
											];
											newUris[uriIndex()] = e.currentTarget.value;
											props.edit.setApp(
												"oauth_redirect_uris",
												newUris,
											);
										}}
									/>
									<button
										onClick={() => {
											const newUris = [
												...props.edit.app.oauth_redirect_uris ?? [],
											];
											newUris.splice(uriIndex(), 1);
											props.edit.setApp(
												"oauth_redirect_uris",
												newUris,
											);
										}}
									>
										remove
									</button>
								</li>
							)}
						</For>
						<li>
							<button
								onClick={() => {
									const newUris = [
										...props.edit.app.oauth_redirect_uris ?? [],
										"",
									];
									props.edit.setApp(
										"oauth_redirect_uris",
										newUris,
									);
								}}
							>
								add uri
							</button>
						</li>
					</ul>
					<br />
					<button onClick={() => props.rotateSecret(props.edit.app.id!)}>
						rotate secret
					</button>
				</div>
			</Show>
			<Show when={activeTab() === "sessions"}>
				<div class="sessions">
					<button onClick={() => props.createSession(props.edit.app.id!)}>
						create session
					</button>
					<SessionList appId={props.edit.app.id!} />
				</div>
			</Show>
		</div>
	);
};

// TODO: make this an actual context menu?
const InviteToRoom = (
	props: { x: number; y: number; app: Application },
) => {
	const api = useApi();
	const api2 = useRooms2();
	const rooms = api2.useList();
	const roomItems = () =>
		rooms.ids.map((id) => api2.get(id) ?? null).filter((r): r is Room =>
			r !== null
		);
	const [menuParentRef, setMenuParentRef] = createSignal<ReferenceElement>();
	const [menuRef, setMenuRef] = createSignal<HTMLElement>();

	createEffect(() => {
		setMenuParentRef({
			getBoundingClientRect: () => ({
				x: props.x,
				y: props.y,
				left: props.x,
				top: props.y,
				right: props.x,
				bottom: props.y,
				width: 0,
				height: 0,
			}),
		});

		props.x;
		props.y;
	});

	const menuFloating = useFloating(() => menuParentRef(), () => menuRef(), {
		middleware: [shift({ mainAxis: true, crossAxis: true, padding: 8 })],
		placement: "right-start",
	});

	const inviteToRoom = (room_id: string) => {
		api.client.http.POST("/api/v1/app/{app_id}/invite", {
			params: { path: { app_id: props.app.id } },
			body: { room_id },
		});
	};

	const u = useCurrentUser();
	const self_id = () => u()?.id;

	return (
		<menu
			class="invite-app"
			style={{
				translate: `${menuFloating.x}px ${menuFloating.y}px`,
			}}
			ref={setMenuRef}
		>
			<For each={roomItems() ?? []} fallback="no rooms?">
				{(r) => (
					<RoomInviteButton
						room={r}
						self_id={self_id}
						onInvite={inviteToRoom}
					/>
				)}
			</For>
		</menu>
	);
};

const RoomInviteButton = (
	props: {
		room: RoomT;
		self_id: () => string | undefined;
		onInvite: (id: string) => void;
	},
) => {
	const perms = usePermissions(
		props.self_id,
		() => props.room.id,
		() => undefined,
	);

	return (
		<button
			onClick={() => props.onInvite(props.room.id)}
			disabled={!perms.has("IntegrationsManage")}
		>
			{props.room.name}
		</button>
	);
};
