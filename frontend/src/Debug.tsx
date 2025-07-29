import { createResource, createSignal, For, Show } from "solid-js";
import { leadingAndTrailing, throttle } from "@solid-primitives/scheduled";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";
import { MessageView } from "./Message.tsx";
import { flags } from "./flags.ts";
import type { Embed, Message } from "sdk";
import { EmbedView } from "./UrlEmbed.tsx";
import { Dropdown } from "./Dropdown.tsx";

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
				<summary>url embedder</summary>
				<UrlEmbedDbg />
			</details>
			<details>
				<summary>dropdown</summary>
				<Dropdown
					selected="foo"
					options={[
						{ item: "foo", label: "foo" },
						{ item: "bar", label: "bar" },
						{ item: "baz", label: "baz" },
					]}
				/>
			</details>
			<details>
				<summary>colors</summary>
				<ul class="debug-colors">
					<li>
						<div class="colored red"></div> red
					</li>
					<li>
						<div class="colored green"></div> green
					</li>
					<li>
						<div class="colored yellow"></div> yellow
					</li>
					<li>
						<div class="colored blue"></div> blue
					</li>
					<li>
						<div class="colored magenta"></div> magenta
					</li>
					<li>
						<div class="colored cyan"></div> cyan
					</li>
					<li>
						<div class="colored orange"></div> orange
					</li>
					<li>
						<div class="colored teal"></div> teal
					</li>
				</ul>
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
			<For each={searchResults.latest as any}>
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
	const [invite] = createResource(inviteCode, async (code) => {
		if (!code) return null;
		const data = await api.invites.fetch(() => code);
		return data;
	});

	return (
		<>
			<label>
				invite code:{" "}
				<input
					type="text"
					onInput={(e) => setInviteCode(e.currentTarget.value)}
				/>
			</label>
			<br />
			<Show when={invite.loading}>loading...</Show>
			<Show when={invite.latest}>
				<pre>{JSON.stringify(invite.latest, null, 4)}</pre>
			</Show>
		</>
	);
};

const UrlEmbedDbg = () => {
	const api = useApi();
	let url: string;
	const [data, setData] = createSignal<Embed | null>(null);

	async function generate(e: SubmitEvent) {
		e.preventDefault();
		if (!url) return;
		const res = await api.client.http.POST("/api/v1/debug/embed-url", {
			body: { url },
		});
		setData(res.data as any);
	}

	return (
		<>
			<form onSubmit={generate}>
				<label>
					url:{" "}
					<input type="url" onInput={(e) => (url = e.currentTarget.value)} />
				</label>
			</form>
			<Show when={data()}>
				<div>
					<EmbedView embed={data()!} />
				</div>
			</Show>
			<pre>{JSON.stringify(data(), null, 4)}</pre>
		</>
	);
};
