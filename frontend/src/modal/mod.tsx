import {
	createEffect,
	createMemo,
	createSignal,
	For,
	type ParentProps,
	Show,
} from "solid-js";
import { type Modal as ContextModal, useCtx } from "../context.ts";
import { autofocus } from "@solid-primitives/autofocus";
import type { Media } from "sdk";
import { getHeight, getUrl, getWidth, Resize } from "../media/util.tsx";
import { useApi } from "../api.tsx";
import { createResource } from "solid-js";
import { MessageView } from "../Message.tsx";
import { diffChars } from "diff";
import { useNavigate } from "@solidjs/router";
import { ModalResetPassword } from "../user_settings/mod.tsx";

export const Modal = (props: ParentProps) => {
	const ctx = useCtx()!;
	return (
		<div class="modal">
			<div class="bg" onClick={() => ctx.dispatch({ do: "modal.close" })}></div>
			<div class="content">
				<div class="base"></div>
				<div class="inner" role="dialog" aria-modal>
					{props.children}
				</div>
			</div>
		</div>
	);
};

export function getModal(modal: ContextModal) {
	switch (modal.type) {
		case "alert": {
			return <ModalAlert text={modal.text} />;
		}
		case "confirm": {
			return <ModalConfirm text={modal.text} cont={modal.cont} />;
		}
		case "prompt": {
			return <ModalPrompt text={modal.text} cont={modal.cont} />;
		}
		case "media": {
			return <ModalMedia media={modal.media} />;
		}
		case "message_edits": {
			return (
				<ModalMessageEdits
					thread_id={modal.thread_id}
					message_id={modal.message_id}
				/>
			);
		}
		case "reset_password": {
			return <ModalResetPassword />;
		}
		case "palette": {
			return <ModalPalette />;
		}
	}
}

const ModalAlert = (props: { text: string }) => {
	const ctx = useCtx()!;
	return (
		<Modal>
			<p>{props.text}</p>
			<div class="bottom">
				<button onClick={() => ctx.dispatch({ do: "modal.close" })}>
					okay!
				</button>
			</div>
		</Modal>
	);
};

const ModalConfirm = (
	props: { text: string; cont: (bool: boolean) => void },
) => {
	const ctx = useCtx()!;
	return (
		<Modal>
			<p>{props.text}</p>
			<div class="bottom">
				<button
					onClick={() => {
						props.cont(true);
						ctx.dispatch({ do: "modal.close" });
					}}
				>
					okay!
				</button>
				<button
					onClick={() => {
						props.cont(false);
						ctx.dispatch({ do: "modal.close" });
					}}
				>
					nevermind...
				</button>
			</div>
		</Modal>
	);
};

const ModalPrompt = (
	props: { text: string; cont: (s: string | null) => void },
) => {
	const ctx = useCtx()!;
	return (
		<Modal>
			<p>{props.text}</p>
			<div style="height: 8px"></div>
			<form
				onSubmit={(e) => {
					e.preventDefault();
					const form = e.target as HTMLFormElement;
					const input = form.elements.namedItem(
						"text",
					) as HTMLInputElement;
					props.cont(input.value);
					ctx.dispatch({ do: "modal.close" });
				}}
			>
				<input type="text" name="text" use:autofocus autofocus />
				<div class="bottom">
					<input type="submit" value="done!"></input>{" "}
					<button
						onClick={() => {
							props.cont(null);
							ctx.dispatch({ do: "modal.close" });
						}}
					>
						nevermind...
					</button>
				</div>
			</form>
		</Modal>
	);
};

// currently only supports images!
// though, it doesn't make much sense for video/audio/other media?
const ModalMedia = (props: { media: Media }) => {
	const ctx = useCtx();

	const [loaded, setLoaded] = createSignal(false);
	const height = () => getHeight(props.media);
	const width = () => getWidth(props.media);

	createEffect(() => console.log("loaded", loaded()));
	return (
		<div class="modal modal-media">
			<div class="bg" onClick={() => ctx.dispatch({ do: "modal.close" })}></div>
			<div class="content">
				<div class="base"></div>
				<div class="inner" role="dialog" aria-modal>
					<Resize height={height()} width={width()}>
						<div class="image full">
							<div class="media-loader" classList={{ loaded: loaded() }}>
								loading
							</div>
							<img
								src={getUrl(props.media)}
								alt={props.media.alt ?? undefined}
								height={height()!}
								width={width()!}
								onLoad={[setLoaded, true]}
								onEmptied={[setLoaded, false]}
							/>
						</div>
					</Resize>
					<a href={props.media.source.url}>Go to url</a>
				</div>
			</div>
		</div>
	);
};

const ModalMessageEdits = (
	props: { thread_id: string; message_id: string },
) => {
	// FIXME: pagination
	const api = useApi();
	const [edits] = createResource(
		{ channel_id: props.thread_id, message_id: props.message_id },
		async (path) => {
			const { data } = await api.client.http.GET(
				"/api/v1/channel/{channel_id}/message/{message_id}/version",
				{
					params: {
						path,
						query: { limit: 100 },
					},
				},
			);
			return data!;
		},
	);

	diffChars;

	return (
		<Modal>
			<h3 style="margin: -8px 6px">edit history</h3>
			<ul class="edit-history">
				<For each={edits()?.items ?? []} fallback={"loading"}>
					{(i, x) => {
						const prev = edits()?.items[x() - 1];
						if (prev) {
							const pages = diffChars(prev.content ?? "", i.content ?? "");
							const content = pages.map((i) => {
								if (i.added) return `<ins>${i.value}</ins>`;
								if (i.removed) return `<del>${i.value}</del>`;
								return i.value;
							}).join("");
							return (
								<li>
									<MessageView message={{ ...i, content }} separate />
								</li>
							);
						} else {
							return (
								<li>
									<MessageView message={i} separate />
								</li>
							);
						}
					}}
				</For>
			</ul>
		</Modal>
	);
};

const ModalPalette = () => {
	const api = useApi();
	const ctx = useCtx();
	const navigate = useNavigate();

	const [query, setQuery] = createSignal("");
	const [selectedIndex, setSelectedIndex] = createSignal(0);

	// try to load all threads
	const rooms = api.rooms.list();
	api.dms.list();
	createEffect(() => {
		for (const room of rooms()?.items ?? []) {
			api.channels.list(() => room.id);
		}
	});

	type PaletteItem = {
		type: "room" | "thread" | "link";
		id: string;
		name: string;
		action: () => void;
	};

	const allItems = createMemo((): PaletteItem[] => {
		const rooms = [...api.rooms.cache.values()].map((room) => ({
			type: "room" as const,
			id: room.id,
			name: room.name,
			action: () => navigate(`/room/${room.id}`),
		}));
		const threads = [...api.channels.cache.values()].map((thread) => ({
			type: "thread" as const,
			id: thread.id,
			name: thread.name,
			action: () => navigate(`/channel/${thread.id}`),
		}));

		const staticItems: PaletteItem[] = [
			{
				type: "link" as const,
				id: "home",
				name: "home",
				action: () => navigate("/"),
			},
			{
				type: "link" as const,
				id: "inbox",
				name: "inbox",
				action: () => navigate("/inbox"),
			},
			{
				type: "link" as const,
				id: "friends",
				name: "friends",
				action: () => navigate("/friends"),
			},
			{
				type: "link" as const,
				id: "settings",
				name: "settings",
				action: () => navigate("/settings"),
			},
		];

		return [...staticItems, ...rooms, ...threads];
	});

	const recentThreads = createMemo(() => {
		return ctx.recentThreads().slice(1).map((i) => api.channels.cache.get(i)!)
			.map((
				thread,
			) => ({
				type: "thread" as const,
				id: thread.id,
				name: thread.name,
				action: () => navigate(`/thread/${thread.id}`),
			}));
	});

	const filteredItems = createMemo(() => {
		const q = query().toLowerCase();
		if (!q) {
			return recentThreads();
		}
		return allItems().filter((item) =>
			item.name && item.name.toLowerCase().includes(q)
		);
	});

	createEffect(() => {
		setSelectedIndex(0);
	});

	const handleKeyDown = (e: KeyboardEvent) => {
		const len = filteredItems().length;
		if (len === 0) return;

		if (e.key === "ArrowDown") {
			e.preventDefault();
			setSelectedIndex((i) => (i + 1) % len);
		} else if (e.key === "ArrowUp") {
			e.preventDefault();
			setSelectedIndex((i) => (i - 1 + len) % len);
		} else if (e.key === "Enter") {
			e.preventDefault();
			const item = filteredItems()[selectedIndex()];
			if (item) {
				item.action();
				ctx.dispatch({ do: "modal.close" });
			}
		}
	};

	const close = () => ctx.dispatch({ do: "modal.close" });

	return (
		<Modal>
			<div onKeyDown={handleKeyDown} class="palette">
				<h3 class="dim">palette</h3>
				<input
					type="text"
					autofocus
					ref={(a) => queueMicrotask(() => a.focus())}
					value={query()}
					onInput={(e) => setQuery(e.currentTarget.value)}
					placeholder="type to search..."
				/>
				<div class="items">
					<For each={filteredItems().slice(0, 10)}>
						{(item, i) => (
							<div
								class="item"
								classList={{ selected: i() === selectedIndex() }}
								onClick={() => {
									item.action();
									close();
								}}
								onMouseEnter={() => setSelectedIndex(i())}
							>
								<span>{item.name}</span>
							</div>
						)}
					</For>
				</div>
			</div>
		</Modal>
	);
};
