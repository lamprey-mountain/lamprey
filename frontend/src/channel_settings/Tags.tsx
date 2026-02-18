import {
	createEffect,
	createResource,
	createSignal,
	For,
	Show,
	type VoidProps,
} from "solid-js";
import { useApi } from "../api.tsx";
import type { Channel, Tag } from "sdk";
import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import { usePermissions } from "../hooks/usePermissions.ts";
import { useModals } from "../contexts/modal";
import icEdit from "../assets/edit.png";
import icDelete from "../assets/delete.png";

export function Tags(props: VoidProps<{ channel: Channel }>) {
	const api = useApi();
	const [, modalCtl] = useModals();
	const perms = usePermissions(
		() => api.users.cache.get("@self")?.id,
		() => props.channel.room_id,
		() => props.channel.id,
	);

	const [tags, { refetch }] = createResource(async () => {
		const { data } = await api.client.http.GET(
			"/api/v1/channel/{channel_id}/tag",
			{ params: { path: { channel_id: props.channel.id } } },
		);
		return data;
	});

	const deleteTag = (tag_id: string) => () => {
		modalCtl.confirm(
			"Are you sure you want to delete this tag?",
			(conf) => {
				if (!conf) return;
				api.channels.deleteTag(props.channel.id, tag_id).then(() => {
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

	const openCreateModal = () => {
		modalCtl.open({
			type: "tag_editor",
			forumChannelId: props.channel.id,
			onSave: () => {
				refetch();
			},
		});
	};

	const openEditModal = (tag: Tag) => {
		modalCtl.open({
			type: "tag_editor",
			tag,
			forumChannelId: props.channel.id,
			onSave: () => {
				refetch();
			},
		});
	};

	return (
		<div class="room-settings-integrations">
			<h2>tags</h2>
			<header class="applications-header">
				<input
					type="search"
					placeholder="search tags"
					aria-label="search tags"
					onInput={(e) => setSearch(e.target.value)}
				/>
				<Show when={perms.has("TagManage")}>
					<button type="button" class="primary big" onClick={openCreateModal}>
						create tag
					</button>
				</Show>
			</header>
			<Show when={tags()}>
				<ul class="tag-list">
					<For
						each={tags()!.items.filter((i) =>
							i.name.toLowerCase().includes(search().toLowerCase())
						)}
					>
						{(tag) => {
							const [tagColor, setTagColor] = createSignal(tag.color);

							createEffect(() => {
								setTagColor(tag.color);
							});

							return (
								<li class="tag-item">
									<div class="tag-content">
										<div
											class="tag-color-indicator"
											style={{
												"background-color": tagColor() ?? "#808080",
											}}
										>
										</div>
										<div class="tag-info">
											<h3 class="name">{tag.name}</h3>
											<div class="description">
												{tag.description ?? "No description"}
											</div>
											<div class="dim">
												{tag.active_thread_count} active threads •{" "}
												{tag.total_thread_count} total threads
												{tag.archived && " • archived"}
												{tag.restricted && " • restricted"}
											</div>
										</div>
									</div>
									<Show when={perms.has("TagManage")}>
										<TagToolbar
											onEdit={() => openEditModal(tag)}
											onDelete={deleteTag(tag.id)}
										/>
									</Show>
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

interface TagToolbarProps {
	onEdit: () => void;
	onDelete: () => void;
}

const TagToolbar = (props: TagToolbarProps) => {
	return (
		<div class="message-toolbar">
			<button
				title="Edit tag"
				aria-label="Edit tag"
				onClick={(e) => {
					e.stopPropagation();
					props.onEdit();
				}}
			>
				<img class="icon" src={icEdit} />
			</button>
			<button
				title="Delete tag"
				aria-label="Delete tag"
				onClick={(e) => {
					e.stopPropagation();
					props.onDelete();
				}}
			>
				<img class="icon" src={icDelete} />
			</button>
		</div>
	);
};
