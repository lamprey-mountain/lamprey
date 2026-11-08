import { throttle } from "@solid-primitives/scheduled";
import { createEffect, createSignal, For, Show } from "solid-js";
import type { Media as MediaT } from "ts-sdk";
import { useMedia } from "@/api";
import { useMenu } from "@/contexts/mod.tsx";
import { formatBytes, getThumbFromId } from "@/media/util.tsx";

export function Media() {
	const { setMenu } = useMenu();
	const media2 = useMedia();
	const [query, setQuery] = createSignal("");
	const [searchResults, setSearchResults] = createSignal<MediaT[]>([]);

	const throttledSearch = throttle(async (q: string) => {
		const results = await media2.search(q.length > 0 ? q : "*");
		if (results && results.results) {
			setSearchResults(
				results.results
					.map((id: string) => media2.cache.get(id))
					.filter(Boolean),
			);
		} else {
			setSearchResults([]);
		}
	}, 500);

	createEffect(() => {
		throttledSearch(query());
	});

	// read from cache so deleted media is dropped from the list
	const items = () =>
		searchResults()
			.map((media) => media2.cache.get(media.id))
			.filter((media) => media !== undefined);

	return (
		<div class="room-settings-members">
			<h2>Media</h2>
			<input
				type="text"
				placeholder="Search media..."
				onInput={(e) => setQuery(e.currentTarget.value)}
			/>
			<header>
				<div class="name">filename</div>
			</header>
			<Show when={items().length > 0}>
				<ul>
					<For each={items()}>
						{(media) => (
							<li>
								<div class="profile" style="flex:1">
									<Show when={media.has_thumbnail} fallback="(media)">
										<img src={getThumbFromId(media.id, 64)} class="avatar" />
									</Show>
									<div>
										<h3 class="name">{media.filename}</h3>
										<div class="dim">
											{formatBytes(media.size)} - {media.content_type} -{" "}
											{media.alt}
										</div>
										<div class="dim">{media.id}</div>
									</div>
								</div>
								<button
									type="button"
									class="button"
									onClick={(e) => {
										e.stopPropagation();
										setMenu({
											type: "media",
											media_id: media.id,
											x: e.clientX,
											y: e.clientY,
											admin: true,
										});
									}}
								>
									options
								</button>
							</li>
						)}
					</For>
				</ul>
			</Show>
		</div>
	);
}
