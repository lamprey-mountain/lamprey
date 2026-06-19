import { throttle } from "@solid-primitives/scheduled";
import { createEffect, createSignal, For, Show } from "solid-js";
import { useMedia } from "@/api";
import { formatBytes, getThumbFromId } from "@/media/util.tsx";
import { Media as MediaT } from "ts-sdk";

export function Media() {
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
			<Show when={searchResults().length > 0}>
				<ul>
					<For each={searchResults()}>
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
								<button type="button" class="button">
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

// TODO: media admin context menu
