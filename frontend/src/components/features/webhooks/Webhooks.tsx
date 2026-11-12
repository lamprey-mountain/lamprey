import { debounce } from "@solid-primitives/scheduled";
import fuzzysort from "fuzzysort";
import type { Channel, Webhook } from "sdk";
import { createUpload, getTimestampFromUUID } from "sdk";
import { createSignal, For, Show } from "solid-js";
import { useApi, useChannels, useUsers } from "@/api";
import { ChannelPicker } from "@/atoms/ChannelPicker";
import { Icon } from "@/atoms/Icon";
import { Savebar } from "@/atoms/Savebar";
import { Search } from "@/atoms/Search";
import { Time } from "@/atoms/Time";
import { createTooltip } from "@/atoms/Tooltip";
import { Avatar, UserDisplayName } from "@/components/shared/User";
import { useConfig } from "@/lib/config";
import { icCheck, icCopy } from "@/utils/icons";
import {
	useWebhooks,
	type WebhookDraft,
	type WebhookScope,
	WebhooksProvider,
} from "./context";

export function WebhooksView(props: { scope: WebhookScope }) {
	const channels = useChannels();
	const roomId = () => {
		if (props.scope.type === "room") return props.scope.room_id;
		return channels.cache.get(props.scope.channel_id)?.room_id;
	};
	const defaultChannelId = () =>
		[...channels.cache.values()].find((channel) => channel.room_id === roomId())
			?.id;

	return (
		<WebhooksProvider scope={props.scope} defaultChannelId={defaultChannelId}>
			<WebhooksInner scope={props.scope} />
		</WebhooksProvider>
	);
}

const draftId = (draft: WebhookDraft) =>
	draft.state === "create" ? draft.nonce : draft.data.id;

const draftName = (draft: WebhookDraft) => {
	if (draft.state === "create") return draft.create.name;
	if (draft.state === "update") {
		return draft.update.name ?? draft.data.name;
	}
	return draft.data.name;
};

const draftAvatar = (draft: WebhookDraft) => {
	if (draft.state === "create") return draft.create.avatar;
	if (draft.state === "update" && draft.update.avatar !== undefined) {
		return draft.update.avatar;
	}
	return draft.data.avatar;
};

const draftChannelId = (draft: WebhookDraft) => {
	if (draft.state === "create") return draft.channel_id;
	if (draft.state === "update" && draft.update.channel_id !== undefined) {
		return draft.update.channel_id;
	}
	return draft.data.channel_id;
};

const draftCreatorId = (draft: WebhookDraft) =>
	draft.state === "create" ? undefined : (draft.data.creator_id ?? undefined);

const draftToken = (draft: WebhookDraft) =>
	draft.state === "create" ? undefined : draft.data.token;

const draftWebhook = (draft: WebhookDraft): Webhook | undefined =>
	draft.state === "create" ? undefined : draft.data;

function CopyUrlButton(props: { url: string | undefined }) {
	const [copied, setCopied] = createSignal(false);
	const clearCopied = debounce(() => setCopied(false), 2000);

	const tooltip = createTooltip({
		tip: () => (
			<div class="copyable2-tip" classList={{ copied: copied() }}>
				<Icon src={copied() ? icCheck : icCopy} color={null} />{" "}
				{copied()
					? "url copied!"
					: props.url
						? "copy webhook url"
						: "save the webhook first to copy its url"}
			</div>
		),
	});

	const copy = () => {
		if (!props.url) return;
		navigator.clipboard.writeText(props.url);
		setCopied(true);
		clearCopied();
	};

	return (
		<span class="webhook-copy" ref={tooltip.content}>
			<button type="button" class="button" disabled={!props.url} onClick={copy}>
				copy url
			</button>
		</span>
	);
}

function WebhookCreator(props: { user_id: string; room_id?: string }) {
	return (
		<span class="mention mention-user">
			{"@"}
			<UserDisplayName
				user_id={props.user_id}
				room_id={props.room_id}
				onClick
			/>
		</span>
	);
}

function WebhooksInner(props: { scope: WebhookScope }) {
	const api = useApi();
	const users = useUsers();
	const config = useConfig();
	const channels = useChannels();
	const webhooks = useWebhooks();

	const [search, setSearch] = createSignal("");

	const roomId = () => {
		if (props.scope.type === "room") return props.scope.room_id;
		return channels.cache.get(props.scope.channel_id)?.room_id;
	};

	const channelList = () =>
		[...channels.cache.values()].filter((c) => c.room_id === roomId());

	const filteredDrafts = () => {
		const query = search();
		const all = webhooks.drafts;
		if (!query) return all;
		const results = fuzzysort.go(query, all, {
			key: (draft) => draftName(draft),
			threshold: -10000,
		});
		return results.map((result) => result.obj);
	};

	const create = () => {
		const id = webhooks.create(channelList()[0]?.id);
		requestAnimationFrame(() => {
			document
				.getElementById(`webhook-${id}`)
				?.scrollIntoView({ behavior: "smooth", block: "center" });
		});
	};

	const webhookUser = (draft: WebhookDraft) => ({
		id: draftId(draft),
		name: draftName(draft),
		avatar: draftAvatar(draft) ?? null,
		banner: null,
		description: null,
		bot: true,
		system: false,
		version_id: "",
		flags: 0,
		presence: { status: "Offline" as const, activities: [] },
		preferences: null,
	});

	const selectedChannel = (draft: WebhookDraft): Channel | null => {
		const id = draftChannelId(draft);
		if (!id) return null;
		return channelList().find((channel) => channel.id === id) ?? null;
	};

	const webhookUrl = (draft: WebhookDraft) => {
		const token = draftToken(draft);
		if (!token) return undefined;
		return `${config.api_url}/api/v1/webhook/${draftId(draft)}/${token}`;
	};

	const setAvatarFile = async (draft: WebhookDraft, file: File) => {
		await createUpload({
			client: api.client,
			file,
			onComplete(media) {
				webhooks.updateDraft(draftId(draft), { avatar: media.id });
			},
			onFail(_error) {},
			onPause() {},
			onResume() {},
			onProgress(_progress) {},
		});
	};

	return (
		<div class="room-settings-integrations">
			<h2>webhooks</h2>
			<header class="applications-header webhooks-header">
				<Search placeholder="search" onInput={setSearch} />
				<button type="button" class="button primary big" onClick={create}>
					create
				</button>
			</header>
			<ul class="webhooks-list">
				<For each={filteredDrafts()}>
					{(draft) => {
						const id = draftId(draft);
						const creator = users.use(() => draftCreatorId(draft));
						const deleted = () => draft.state === "delete";
						const created = () => draftWebhook(draft);
						const draftState = () =>
							draft.state === "clean" ? undefined : draft.state;
						let avatarInputEl: HTMLInputElement | undefined;

						const openAvatarPicker = () => {
							if (deleted()) return;
							avatarInputEl?.click();
						};

						return (
							<li
								id={`webhook-${id}`}
								class="webhook-item"
								data-draft-state={draftState()}
							>
								<div class="webhook-avatar">
									<div class="avatar-uploader" onClick={openAvatarPicker}>
										<div class="avatar-inner">
											<Avatar user={webhookUser(draft)} />
											<div class="overlay">upload avatar</div>
										</div>
										<Show when={draftAvatar(draft)}>
											<button
												type="button"
												class="remove"
												onClick={(e) => {
													e.stopPropagation();
													webhooks.updateDraft(id, { avatar: null });
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
												const file = e.target.files?.[0];
												if (file) setAvatarFile(draft, file);
											}}
										/>
									</div>
								</div>
								<div class="webhook-main">
									<h3 class="name webhook-name">{draftName(draft)}</h3>
									<div class="webhook-fields">
										<div>
											<h3 class="dim">name</h3>
											<input
												type="text"
												value={draftName(draft)}
												disabled={deleted()}
												onInput={(e) =>
													webhooks.updateDraft(id, {
														name: e.target.value,
													})
												}
											/>
										</div>
										<div>
											<h3 class="dim">channel</h3>
											<ChannelPicker
												selected={selectedChannel(draft)}
												channels={channelList}
												onSelect={(channel) => {
													if (channel && !deleted()) {
														webhooks.updateDraft(id, {
															channel_id: channel.id,
														});
													}
												}}
											/>
										</div>
									</div>
									<div class="webhook-bottom">
										<div class="webhook-created">
											<Show
												when={created()}
												fallback={<span class="dim">not saved yet</span>}
											>
												{(webhook) => (
													<>
														<span class="dim">created</span>
														<Time date={getTimestampFromUUID(webhook().id)} />
														<Show when={draftCreatorId(draft)}>
															{(creatorId) => (
																<>
																	<span class="dim">by</span>
																	<WebhookCreator
																		user_id={creatorId()}
																		room_id={roomId() ?? undefined}
																	/>
																</>
															)}
														</Show>
													</>
												)}
											</Show>
										</div>
										<div class="webhook-actions">
											<CopyUrlButton url={webhookUrl(draft)} />
											<Show
												when={deleted()}
												fallback={
													<button
														type="button"
														onClick={() => webhooks.remove(id)}
														class="button danger"
													>
														delete
													</button>
												}
											>
												<button
													type="button"
													onClick={() => webhooks.restore(id)}
													class="button"
												>
													undo delete
												</button>
											</Show>
										</div>
									</div>
								</div>
							</li>
						);
					}}
				</For>
			</ul>
			<Savebar
				show={webhooks.dirty}
				saving={webhooks.saving}
				onCancel={webhooks.reset}
				onSave={webhooks.save}
			/>
		</div>
	);
}
