import { createMemo, createSignal, For, Show } from "solid-js";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";
import { ChannelIcon } from "./User.tsx";
import { useNavigate } from "@solidjs/router";
import type { Channel } from "sdk";
import { useModals } from "./contexts/modal.tsx";

export const ThreadPopout = (props: { channel_id: string }) => {
	const api = useApi();
	const ctx = useCtx();
	const navigate = useNavigate();
	const [search, setSearch] = createSignal("");

	const activeThreads = api.threads.listForChannel(() => props.channel_id);
	const archivedThreads = api.threads.listArchivedForChannel(
		() => props.channel_id,
	);

	const sortedThreads = createMemo(() => {
		const active = activeThreads()?.items ?? [];
		const archived = archivedThreads()?.items ?? [];

		const query = search().toLowerCase();
		const filter = (t: Channel) => t.name.toLowerCase().includes(query);

		const joined: Channel[] = [];
		const notJoined: Channel[] = [];

		for (const t of active) {
			// FIXME: populate thread_member for joined threads
			if (filter(t)) {
				if (t.thread_member && t.thread_member.membership === "Join") {
					joined.push(t);
				} else {
					notJoined.push(t);
				}
			}
		}

		const filteredArchived = archived.filter(filter);

		return {
			joined,
			notJoined,
			archived: filteredArchived,
		};
	});

	const [, modalctl] = useModals();
	const nav = useNavigate();
	const onCreateThread = () => {
		const channel_id = props.channel_id;
		const channel = api.channels.cache.get(channel_id)!;
		ctx.setThreadsView(null);
		modalctl.prompt("name?", async (name) => {
			if (!name) return;
			const chan = await api.channels.create(channel.room_id!, {
				name,
				parent_id: channel_id,
				type: "ThreadPublic",
			});
			nav(`/channel/${chan.id}`);
		});
	};

	const onThreadClick = (thread: Channel) => {
		navigate(`/thread/${thread.id}`);
		ctx.setThreadsView(null);
	};

	// TODO: show skeleton ui when loading threads

	return (
		<div class="threads-popout" onClick={(e) => e.stopPropagation()}>
			<div class="header">
				<input
					type="search"
					placeholder="Search threads..."
					value={search()}
					onInput={(e) => setSearch(e.currentTarget.value)}
					class="search-pad"
				/>
				<button class="primary" onClick={onCreateThread}>create thread</button>
			</div>
			<div class="thread-list">
				<Show when={sortedThreads().joined.length}>
					<h3 class="dim">joined threads</h3>
				</Show>
				<For each={sortedThreads().joined}>
					{(thread) => (
						<div
							class="thread-item"
							onClick={() => onThreadClick(thread)}
						>
							<ChannelIcon channel={thread} />
							<span>{thread.name}</span>
							<span class="badge">Joined</span>
						</div>
					)}
				</For>
				<Show when={sortedThreads().notJoined.length}>
					<h3 class="dim">active threads</h3>
				</Show>
				<For each={sortedThreads().notJoined}>
					{(thread) => (
						<div
							class="thread-item"
							onClick={() => onThreadClick(thread)}
						>
							<ChannelIcon channel={thread} />
							<span>{thread.name}</span>
						</div>
					)}
				</For>
				<Show when={sortedThreads().archived.length}>
					<h3 class="dim">archived threads</h3>
				</Show>
				<Show when={sortedThreads().archived.length > 0}>
					<For each={sortedThreads().archived}>
						{(thread) => (
							<div
								class="thread-item"
								onClick={() => onThreadClick(thread)}
							>
								<ChannelIcon channel={thread} />
								<span>{thread.name}</span>
							</div>
						)}
					</For>
					<Show
						when={(sortedThreads().joined.length +
							sortedThreads().notJoined.length +
							sortedThreads().archived.length) === 0}
					>
						<div style="text-align:center">no threads :(</div>
					</Show>
				</Show>
			</div>
		</div>
	);
};
