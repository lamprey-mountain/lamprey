import {
	createEffect,
	createResource,
	createSignal,
	For,
	onCleanup,
	Show,
	type VoidProps,
} from "solid-js";
import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import type { Channel } from "sdk";
import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import { Avatar } from "../User.tsx";
import { Time } from "../Time.tsx";
import { useFloating } from "solid-floating-ui";
import { ReferenceElement, shift } from "@floating-ui/dom";
import { usePermissions } from "../hooks/usePermissions.ts";
import { createUpload, getTimestampFromUUID, type Webhook } from "sdk";
import { Dropdown } from "../Dropdown.tsx";
import { useConfig } from "../config.tsx";
import { useModals } from "../contexts/modal";

export function Webhooks(props: VoidProps<{ channel: Channel }>) {
	const ctx = useCtx();
	const api = useApi();
	const config = useConfig();
	const [, modalCtl] = useModals();

	const [webhooks, { refetch }] = createResource(async () => {
		const { data } = await api.client.http.GET(
			"/api/v1/channel/{channel_id}/webhook",
			{ params: { path: { channel_id: props.channel.id } } },
		);
		return data;
	});

	const removeWebhook = (webhook_id: string) => () => {
		modalCtl.confirm(
			"Are you sure you want to delete this webhook?",
			(conf) => {
				if (!conf) return;
				api.client.http.DELETE(
					"/api/v1/webhook/{webhook_id}",
					{ params: { path: { webhook_id } } },
				).then(() => {
					refetch();
				});
			},
		);
	};

	const fetchMore = () => {
		// TODO: make this work
	};

	const [bottom, setBottom] = createSignal<Element | undefined>();

	createIntersectionObserver(() => bottom() ? [bottom()!] : [], (entries) => {
		for (const entry of entries) {
			if (entry.isIntersecting) fetchMore();
		}
	});

	const [search, setSearch] = createSignal("");
	// TODO: use fuzzysort here

	const create = () => {
		modalCtl.prompt("New webhook name?", async (name) => {
			if (!name) return;
			await api.client.http.POST("/api/v1/channel/{channel_id}/webhook", {
				params: { path: { channel_id: props.channel.id } },
				body: {
					name,
				},
			});
			refetch();
		});
	};

	return (
		<div class="room-settings-integrations">
			<h2>webhooks</h2>
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
			<Show when={webhooks()}>
				<ul>
					<For
						each={webhooks()!.items.filter((i) =>
							i.name.toLowerCase().includes(search().toLowerCase())
						)}
					>
						{(i) => {
							const creator = api.users.fetch(() => i.creator_id);
							const [name, setName] = createSignal(i.name);
							const [avatar, setAvatar] = createSignal(i.avatar);

							const webhookUser = () => ({
								id: i.id,
								name: i.name,
								avatar: avatar(),
								banner: null,
								description: null,
								flags: 0,
								presence: { status: "Offline" as const, activities: [] },
								relationship: null,
								user_config: null,
							});

							const channels = api.channels.list(() => i.room_id);

							createEffect(() => {
								setName(i.name);
							});

							const updateWebhook = async (channel_id: string | null) => {
								console.log(channel_id);
								if (!channel_id) return;
								return;
								await api.client.http.PATCH("/api/v1/webhook/{webhook_id}", {
									params: { path: { webhook_id: i.id } },
									body: {
										name: name(),
									},
								});
							};

							const copyWebhookUrl = () => {
								const webhookUrl =
									`${config.api_url}/api/v1/webhook/${i.id}/${i.token}`;
								navigator.clipboard.writeText(webhookUrl);
							};

							const setAvatarFile = async (f: File) => {
								await createUpload({
									client: api.client,
									file: f,
									onComplete(media) {
										setAvatar(media.id);
										api.client.http.PATCH("/api/v1/webhook/{webhook_id}", {
											params: { path: { webhook_id: i.id } },
											body: {
												avatar: media.id,
											},
										});
									},
									onFail(_error) {},
									onPause() {},
									onResume() {},
									onProgress(_progress) {},
								});
							};

							const removeAvatar = async () => {
								setAvatar(null);
								await api.client.http.PATCH("/api/v1/webhook/{webhook_id}", {
									params: { path: { webhook_id: i.id } },
									body: {
										avatar: null,
									},
								});
							};

							let avatarInputEl!: HTMLInputElement;

							const openAvatarPicker = () => {
								avatarInputEl?.click();
							};

							return (
								<li>
									<details>
										<summary>
											<div style="display: flex; align-items: top; gap: 8px;">
												<Avatar user={webhookUser()} />
												<div>
													<h3 class="name">{name()}</h3>
													<div>
														created on{" "}
														<Time date={getTimestampFromUUID(i.id)} /> by{" "}
														{creator()?.name}
													</div>
												</div>
											</div>
										</summary>
										<div class="info">
											<div style="display: flex; align-items: center; gap: 8px;">
												<div class="avatar-uploader" onClick={openAvatarPicker}>
													<div class="avatar-inner">
														<Avatar user={webhookUser()} />
														<div class="overlay">upload avatar</div>
													</div>
													<Show when={webhookUser().avatar}>
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
													<div style="display: flex;flex: 1; gap: 8px">
														<div style="flex: 1;">
															<h3 class="dim">name</h3>
															<input
																type="text"
																value={name()}
																onInput={(e) => setName(e.target.value)}
																onBlur={updateWebhook}
															/>
														</div>
														<Show when={false}>
															{/* FIXME: changing the channel of webhook */}
															{/* for some reaosn, onSelect doesnt call the fn with the value, only undefined */}
															<div style="flex: 1;">
																<h3 class="dim">channel</h3>
																<Dropdown
																	selected={i.channel_id}
																	onSelect={(i) => console.log(i)}
																	options={channels()?.items?.map((ch) => ({
																		label: ch.name ||
																			`#${ch.id.substring(0, 8)}...`,
																		value: ch.id,
																	})) || []}
																/>
															</div>
														</Show>
													</div>
													<div style="margin-top: 8px; display: flex; gap: 8px">
														<button onClick={copyWebhookUrl}>
															copy url
														</button>
														<button
															onClick={removeWebhook(i.id)}
															class="destructive"
														>
															delete
														</button>
													</div>
												</div>
											</div>
										</div>
									</details>
								</li>
							);
						}}
					</For>
				</ul>
				<div ref={setBottom}></div>
			</Show>
		</div>
	);
}
