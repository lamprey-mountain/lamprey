import { leadingAndTrailing, throttle } from "@solid-primitives/scheduled";
import type { Embed, Message } from "sdk";
import { createResource, createSignal, For, type JSX, Show } from "solid-js";
import { useApi, useMessages } from "@/api";
import { Dropdown } from "./atoms/Dropdown.tsx";
import { MessageView } from "./components/features/chat/Message.tsx";
import { flags } from "./flags.ts";
import { EmbedView } from "./UrlEmbed.tsx";

// @ts-expect-error
const packageJson = __VITE_PACKAGE_JSON__;

// @ts-expect-error
const gitCommit = __VITE_GIT_COMMIT__;

// @ts-expect-error
const gitDirty = __VITE_GIT_DIRTY__;

export const Debug = (): JSX.Element => {
	return (
		<div class="debug">
			<h3>area 51</h3>
			<details>
				<summary>build info</summary>
				commit {gitCommit} {gitDirty && "(dirty)"}
				<pre>
					<code>{JSON.stringify(packageJson, null, 4)}</code>
				</pre>
			</details>
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
	const messagesService = useMessages();
	const [searchQuery, setSearchQueryRaw] = createSignal<string>("");
	const setSearchQuery = leadingAndTrailing(throttle, setSearchQueryRaw, 300);
	const [searchResults] = createResource(searchQuery, async (query: string) => {
		if (!query) return;
		const data = await messagesService.search({ query });
		if (data && "items" in data) {
			return (data as { items: Message[] }).items;
		}
		return [];
	});

	return (
		<>
			<label>
				search messages:{" "}
				<input type="text" onInput={(e) => setSearchQuery(e.target.value)} />
			</label>
			<br />
			<Show when={searchResults.loading}>loading...</Show>
			<For each={searchResults.latest}>
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
	const api2 = useApi();
	const [inviteCode, setInviteCodeRaw] = createSignal<string>("");
	const setInviteCode = leadingAndTrailing(throttle, setInviteCodeRaw, 300);
	const [invite] = createResource(inviteCode, async (code) => {
		if (!code) return null;
		const data = await api2.invites.fetch(code);
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
	const api2 = useApi();
	const [url, setUrl] = createSignal("");
	const [data, setData] = createSignal<Embed | null>(null);
	const [error, setError] = createSignal<{ error: string } | null>(null);

	async function generate(e: SubmitEvent) {
		e.preventDefault();
		const u = url();
		if (!u) return;
		const { data, error } = await api2.client.http.POST(
			"/api/v1/unfurler/debug",
			{
				body: { url: u },
			},
		);
		setData(data?.embeds[0] ?? null);
		setError(error ?? null);
	}

	return (
		<>
			<form onSubmit={generate}>
				<label>
					url:{" "}
					<input type="url" onInput={(e) => setUrl(e.currentTarget.value)} />
				</label>
			</form>
			<Show when={error()}>
				<div style="border: solid red 1px;padding: 4px;background: #ff000044;">
					<b>Error:</b> {error()?.error}
				</div>
			</Show>
			<Show when={data()}>
				{(d) => (
					<div>
						<EmbedView embed={d()} />
					</div>
				)}
			</Show>
			<pre>{JSON.stringify(data(), null, 4)}</pre>
		</>
	);
};
