import { createResource, createSignal, For, Show } from "solid-js";
import { leadingAndTrailing, throttle } from "@solid-primitives/scheduled";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";
import { MessageView } from "./Message.tsx";
import { flags } from "./flags.ts";
import { Message } from "sdk";
import { getThumb, getUrl } from "./media/util.tsx";
import { ImageView } from "./media/mod.tsx";

export const Debug = () => {
	return (
		<div class="debug">
			<h3>area 51</h3>
			<details>
				<summary>invite json</summary>
				<InviteView />
			</details>
			<Show when={flags.has("message_search")}>
				<details>
					<summary>message search</summary>
					<Search />
				</details>
			</Show>
			<details>
				<summary>resizing</summary>
				<div class="dbg-resize">
					<div class="inner">
						<div class="main"></div>
					</div>
				</div>
			</details>
			<details open>
				<summary>url embedder</summary>
				<UrlEmbed />
			</details>
		</div>
	);
};

const Search = () => {
	const ctx = useCtx();
	const [searchQuery, setSearchQueryRaw] = createSignal<string>("");
	const setSearchQuery = leadingAndTrailing(throttle, setSearchQueryRaw, 300);
	const [searchResults] = createResource(
		searchQuery as any,
		(async (query: string) => {
			if (!query) return;
			const { data, error } = await ctx.client.http.POST(
				"/api/v1/search/message",
				{
					body: { query },
				},
			);
			if (error) throw new Error(error);
			return data.items;
		}) as any,
	);

	return (
		<>
			<label>
				search messages:{" "}
				<input type="text" onInput={(e) => setSearchQuery(e.target.value)} />
			</label>
			<br />
			<Show when={searchResults.loading}>loading...</Show>
			<For each={searchResults() as any}>
				{(m: Message) => (
					<li class="message menu-message" data-message-id={m.id}>
						<MessageView message={m} />
					</li>
				)}
			</For>
		</>
	);
};

const InviteView = () => {
	const api = useApi();
	const [inviteCode, setInviteCodeRaw] = createSignal<string>("");
	const setInviteCode = leadingAndTrailing(throttle, setInviteCodeRaw, 300);
	const invite = inviteCode() !== ""
		? api.invites.fetch(inviteCode)
		: () => null;

	return (
		<>
			<label>
				invite code:{" "}
				<input type="text" onInput={(e) => setInviteCode(e.target.value)} />
			</label>
			<br />
			<Show when={invite.loading}>loading...</Show>
			<pre>
				{JSON.stringify(invite(), null, 4)}
			</pre>
		</>
	);
};

const UrlEmbed = () => {
	const api = useApi();
	let url: string;
	const [data, setData] = createSignal({
		"url": "https://git.celery.eu.org/cetahe/cetahe/issues/97",
		"canonical_url": null,
		"title": "url previews",
		// "description": "cetahe - extremely work in progress",
		"description":
			"cetahe - extremely work in progress cetahe - extremely work in progress cetahe - extremely work in progress cetahe - extremely work in progress cetahe - extremely work in progress cetahe - extremely work in progress cetahe - extremely work in progress cetahe - extremely work in progress ",
		"color": null,
		"media": {
			"id": "01952dbb-38ff-79b3-84e8-fe14cef24f0b",
			"filename": "summary-card",
			"alt":
				'Summary card of an issue titled "url previews" in repository cetahe/cetahe',
			"source": {
				"type": "Image",
				"height": 600,
				"width": 1200,
				"language": null,
				"url":
					"http://melon:3900/chat-files/media/01952dbb-38ff-79b3-84e8-fe14cef24f0b?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=GKdf84149deb6f0ff9a7f7323d%2F20250222%2Fgarage%2Fs3%2Faws4_request&X-Amz-Date=20250222T125653Z&X-Amz-Expires=86400&X-Amz-SignedHeaders=host&X-Amz-Signature=2dd1f877e64ccb4df981ab298bbbcbaa595d85809b55150bff01b069bdc6b619",
				"size_unit": "Bytes",
				"size": 46408,
				"mime": "image/png; charset=binary",
				"source": {
					"Downloaded": {
						"source_url":
							"https://git.celery.eu.org/cetahe/cetahe/issues/97/summary-card",
					},
				},
			},
			"tracks": [{
				"type": "Thumbnail",
				"height": 32,
				"width": 64,
				"language": null,
				"url":
					"http://melon:3900/chat-files/thumb/01952dbb-38ff-79b3-84e8-fe14cef24f0b/64x64?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=GKdf84149deb6f0ff9a7f7323d%2F20250222%2Fgarage%2Fs3%2Faws4_request&X-Amz-Date=20250222T125653Z&X-Amz-Expires=86400&X-Amz-SignedHeaders=host&X-Amz-Signature=3d1c032982cc193813c44b3686f978d6b50d8de3e6b31d88a85022c9267949c3",
				"size_unit": "Bytes",
				"size": 479,
				"mime": "image/avif",
				"source": "Generated",
			}, {
				"type": "Thumbnail",
				"height": 160,
				"width": 320,
				"language": null,
				"url":
					"http://melon:3900/chat-files/thumb/01952dbb-38ff-79b3-84e8-fe14cef24f0b/320x320?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=GKdf84149deb6f0ff9a7f7323d%2F20250222%2Fgarage%2Fs3%2Faws4_request&X-Amz-Date=20250222T125653Z&X-Amz-Expires=86400&X-Amz-SignedHeaders=host&X-Amz-Signature=86548ad6cac8fc38b3b33576736c052a396f1a0e17786025209a02add132ecaa",
				"size_unit": "Bytes",
				"size": 2424,
				"mime": "image/avif",
				"source": "Generated",
			}, {
				"type": "Thumbnail",
				"height": 320,
				"width": 640,
				"language": null,
				"url":
					"http://melon:3900/chat-files/thumb/01952dbb-38ff-79b3-84e8-fe14cef24f0b/640x640?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=GKdf84149deb6f0ff9a7f7323d%2F20250222%2Fgarage%2Fs3%2Faws4_request&X-Amz-Date=20250222T125653Z&X-Amz-Expires=86400&X-Amz-SignedHeaders=host&X-Amz-Signature=455de4c1878c56fb60ae83805f03425296a837f0aa11a2f021980d36f46fa1e0",
				"size_unit": "Bytes",
				"size": 6040,
				"mime": "image/avif",
				"source": "Generated",
			}],
		},
		"media_is_thumbnail": true,
		"author_url": null,
		"author_name": null,
		"author_avatar": null,
		"site_name": "gothib",
		"site_avatar": null,
	});

	setData({
		"url": "https://git.celery.eu.org/cetahe/cetahe/issues/97",
		"canonical_url": null,
		"title": "url previews",
		// "description": "cetahe - extremely work in progress",
		"color": null,
		"media": {
			"id": "01952dd4-e382-7151-9aab-b7b739144401",
			"filename": "summary-card",
			"alt":
				'Summary card of an issue titled "url previews" in repository cetahe/cetahe',
			"source": {
				"type": "Image",
				"height": 600,
				"width": 1200,
				"language": null,
				"url":
					"http://melon:3900/chat-files/media/01952dd4-e382-7151-9aab-b7b739144401?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=GKdf84149deb6f0ff9a7f7323d%2F20250222%2Fgarage%2Fs3%2Faws4_request&X-Amz-Date=20250222T132458Z&X-Amz-Expires=86400&X-Amz-SignedHeaders=host&X-Amz-Signature=f26f17ea5f2ba809526c683bb76d20e1d19bc80f470afc6590aa303a92575abc",
				"size_unit": "Bytes",
				"size": 46408,
				"mime": "image/png; charset=binary",
				"source": {
					"Downloaded": {
						"source_url":
							"https://git.celery.eu.org/cetahe/cetahe/issues/97/summary-card",
					},
				},
			},
			"tracks": [{
				"type": "Thumbnail",
				"height": 320,
				"width": 640,
				"language": null,
				"url":
					"http://melon:3900/chat-files/thumb/01952dd4-e382-7151-9aab-b7b739144401/640x640?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=GKdf84149deb6f0ff9a7f7323d%2F20250222%2Fgarage%2Fs3%2Faws4_request&X-Amz-Date=20250222T132458Z&X-Amz-Expires=86400&X-Amz-SignedHeaders=host&X-Amz-Signature=6e20a17d5b3053a6470930dd2b0ea2a49b39a66c4b2c6880134e9d6c6fa51e55",
				"size_unit": "Bytes",
				"size": 6040,
				"mime": "image/avif",
				"source": "Generated",
			}, {
				"type": "Thumbnail",
				"height": 160,
				"width": 320,
				"language": null,
				"url":
					"http://melon:3900/chat-files/thumb/01952dd4-e382-7151-9aab-b7b739144401/320x320?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=GKdf84149deb6f0ff9a7f7323d%2F20250222%2Fgarage%2Fs3%2Faws4_request&X-Amz-Date=20250222T132458Z&X-Amz-Expires=86400&X-Amz-SignedHeaders=host&X-Amz-Signature=a2531cbae435161eee55b5a4781877513143839a0bcb9cccbb64326b40280eae",
				"size_unit": "Bytes",
				"size": 2424,
				"mime": "image/avif",
				"source": "Generated",
			}, {
				"type": "Thumbnail",
				"height": 32,
				"width": 64,
				"language": null,
				"url":
					"http://melon:3900/chat-files/thumb/01952dd4-e382-7151-9aab-b7b739144401/64x64?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=GKdf84149deb6f0ff9a7f7323d%2F20250222%2Fgarage%2Fs3%2Faws4_request&X-Amz-Date=20250222T132458Z&X-Amz-Expires=86400&X-Amz-SignedHeaders=host&X-Amz-Signature=b7cdf3b6203c47dd60b7f9c279e215416497fb45a48e192e5d1c0db768553c56",
				"size_unit": "Bytes",
				"size": 479,
				"mime": "image/avif",
				"source": "Generated",
			}],
		},
		"media_is_thumbnail": false,
		"author_url": null,
		"author_name": null,
		"author_avatar": null,
		"site_name": "gothib",
		"site_avatar": null,
		"description":
			"cetahe - extremely work in progress cetahe - extremely work in progress cetahe - extremely work in progress cetahe - extremely work in progress cetahe - extremely work in progress cetahe - extremely work in progress cetahe - extremely work in progress cetahe - extremely work in progress ",
	});

	async function generate(e: SubmitEvent) {
		e.preventDefault();
		if (!url) return;
		const { data } = await api.client.http.POST("/api/v1/debug/embed-url", {
			body: { url },
		});
		setData(data as any);
	}

	return (
		<>
			<form onSubmit={generate}>
				<label>
					url: <input type="url" onInput={(e) => url = e.target.value} />
				</label>
			</form>
			<div>
				<article class="embed">
					<Show when={data().title}>
						<div class="info">
							<header class="title">
								<a href={data().url}>{data().title}</a>
								<span class="site">
									{" - "}
									{data().site_name}
								</span>
							</header>
							<p class="description">{data().description}</p>
						</div>
					</Show>
					<Show when={data().media && data().media_is_thumbnail}>
						<div class="thumb">
							<ImageView
								media={data().media!}
								thumb_width={64}
								thumb_height={64}
							/>
						</div>
					</Show>
					<Show when={data().media && !data().media_is_thumbnail}>
						<div class="media">
							<ImageView
								media={data().media!}
								thumb_width={320}
								thumb_height={320}
							/>
						</div>
					</Show>
				</article>
			</div>
			<pre>{JSON.stringify(data(), null, 4)}</pre>
		</>
	);
};
