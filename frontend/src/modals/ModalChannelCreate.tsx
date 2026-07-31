import {
	createMemo,
	createSignal,
	For,
	Match,
	onCleanup,
	Show,
	Switch,
} from "solid-js";
import type { ChannelType } from "ts-sdk";
import { useApi } from "@/api";
import { CheckboxOptionWithLabel } from "@/atoms/CheckboxOption";
import { channelIcon } from "@/avatar/ChannelIcon";
import { useCurrentUser } from "@/contexts/currentUser";
import type { ChannelTypeOption } from "@/contexts/modal";
import { useModals } from "@/contexts/modal";
import { createResizeTransition } from "@/hooks/createResizeTransition";
import { flags } from "@/lib/flags";
import { Modal } from "./mod";

interface ModalChannelCreateProps {
	room_id: string;
}

export const ModalChannelCreate = (props: ModalChannelCreateProps) => {
	const api = useApi();
	const [, modalCtl] = useModals();

	const [channelName, setChannelName] = createSignal("");
	const [channelType, setChannelType] = createSignal<ChannelTypeOption | null>(
		null,
	);
	const [channelPrivate, setChannelPrivate] = createSignal(false);
	const [loading, setLoading] = createSignal(false);
	const getMe = useCurrentUser();

	const handleSubmit = async (e: SubmitEvent) => {
		e.preventDefault();

		const name = channelName().trim();
		const type = channelType();
		const me = getMe();
		if (!name || !type || !me) return;

		setLoading(true);
		await api.client.http.POST("/api/v1/room/{room_id}/channel", {
			params: {
				path: { room_id: props.room_id },
			},
			body: {
				name,
				type,
				// // TODO: private channels
				// permission_overwrites: [
				// 	{ type: "Role", id: props.room_id, allow: [], deny: ["ChannelView"] },
				// 	// TODO: only pass if denying @everyone would cause issues
				// 	{ type: "User", id: me.id, allow: ["ChannelView"], deny: [] },
				// ],
			},
		});
		modalCtl.close();
	};

	const handleCancel = () => {
		modalCtl.close();
	};

	const channelTypes = createMemo(
		() =>
			[
				{
					label: "text channel",
					type: "Text",
					description: "instant messaging",
				},
				{
					label: "voice channel",
					type: "Voice",
					description: "connect and talk",
				},
				{
					label: "category channel",
					type: "Category",
					description: "group other channels",
				},
				...(flags.has("channel_forum")
					? [
							{
								label: "forum channel",
								type: "Forum",
								description: "thread only channel",
							},
							{
								label: "forum2 channel",
								type: "Forum2",
								description: "tree style comments forum",
							},
						]
					: []),
				...(flags.has("channel_tickets")
					? [
							{
								label: "ticket channel",
								type: "Ticket",
								description: "private threads forum",
							},
						]
					: []),
				...(flags.has("channel_calendar")
					? [
							{
								label: "calendar channel",
								type: "Calendar",
								description: "experiment, may be removed later",
							},
						]
					: []),
				...(flags.has("channel_documents")
					? [
							{
								label: "document channel",
								type: "Document",
								description: "a single document",
							},
							{
								label: "wiki channel",
								type: "Wiki",
								description: "collection of documents",
							},
						]
					: []),
				...(flags.has("channel_script")
					? [
							{
								label: "scripts channel",
								type: "Scripts",
								description: "experimental arbitrary scripts",
							},
						]
					: []),
			] as { label: string; type: ChannelType; description: string }[],
	);

	const resizeTn = createResizeTransition();

	return (
		<Modal
			class="modal-new-channel unpadded"
			contentRef={(el) => {
				resizeTn.container(el);
				resizeTn.content(el.querySelector(".inner")!);
			}}
		>
			<Switch>
				<Match when={channelType() === null}>
					<div class="main">
						<ul class="channel-type-options">
							<For each={channelTypes()}>
								{(c) => (
									<li>
										<button
											class="button channel-type-option"
											onClick={[setChannelType, c.type]}
										>
											<ChannelTypeIcon channelType={c.type} />

											<div>
												<div>{c.label}</div>
												<div class="dim">{c.description}</div>
											</div>
										</button>
									</li>
								)}
							</For>
						</ul>
					</div>
					<div class="bottom">
						<button
							type="button"
							class="button link link-dim"
							onClick={handleCancel}
						>
							Cancel
						</button>
					</div>
				</Match>
				<Match when={channelType()}>
					{(channelType) => (
						<>
							<div class="main">
								<h3 class="header">
									<ChannelTypeIcon channelType={channelType()} />
									<Show
										when={channelName()}
										fallback={<em class="unnamed">new-channel</em>}
									>
										{(name) => <>{name()}</>}
									</Show>
								</h3>
								<form onSubmit={handleSubmit}>
									<label style="display: block; margin-top: 12px">
										<h3 class="dim" style="margin: 0 2px">
											channel name
										</h3>
										<input
											type="text"
											value={channelName()}
											onInput={(e) => setChannelName(e.currentTarget.value)}
											placeholder="talking"
											required
											ref={(el) => queueMicrotask(() => el.focus())}
											disabled={loading()}
										/>
									</label>

									<Show when={flags.has("channel_create_private")}>
										<CheckboxOptionWithLabel
											id="channel-private"
											checked={channelPrivate()}
											onChange={setChannelPrivate}
											seed="channel-private"
											label="private channel"
											description="private channel"
											disabled={loading()}
										/>
									</Show>
								</form>
							</div>
							<div class="bottom">
								<button
									type="button"
									class="button link link-dim"
									onClick={[setChannelType, null]}
									disabled={loading()}
								>
									Back
								</button>
								<button
									type="submit"
									class="button primary"
									disabled={loading()}
								>
									{loading() ? "Creating..." : "Create"}
								</button>
							</div>
						</>
					)}
				</Match>
			</Switch>
		</Modal>
	);
};

const ChannelTypeIcon = (props: { channelType: ChannelType }) => {
	return (
		<svg aria-hidden="true" class="icon2" viewBox="0 0 64 64">
			<defs>
				<mask id="nsfw">
					<rect width="64" height="64" x="0" y="0" fill="white" />
					<rect rx="4" width="32" height="32" x="32" y="0" fill="black" />
				</mask>
			</defs>
			<g
			// mask={props.channel.nsfw ? "url(#nsfw)" : undefined}
			>
				<rect
					width="64"
					height="64"
					x="0"
					y="0"
					class="inner"
					mask={`url(${channelIcon(props.channelType, "new")})`}
				/>
			</g>
		</svg>
	);

	// <Show when={false}>
	// 	<image href={icChanNsfw} />
	// </Show>
};
