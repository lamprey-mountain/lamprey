import { createResource, createSignal, For, Show } from "solid-js";
import { leadingAndTrailing, throttle } from "@solid-primitives/scheduled";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";
import { MessageView } from "./Message.tsx";
import { flags } from "./flags.ts";
import { Message, UrlEmbed } from "sdk";
import { UrlEmbedView } from "./UrlEmbed.tsx";

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
				<UrlEmbedDbg />
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

const UrlEmbedDbg = () => {
	const api = useApi();
	let url: string;
	const [data, setData] = createSignal<UrlEmbed | null>(null);

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
			<Show when={data()}>
				<div>
					<UrlEmbedView embed={data()!} />
				</div>
			</Show>
			<pre>{JSON.stringify(data(), null, 4)}</pre>
		</>
	);
};
