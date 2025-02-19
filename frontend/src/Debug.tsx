import { createResource, createSignal, For, Show } from "solid-js";
import { leadingAndTrailing, throttle } from "@solid-primitives/scheduled";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";
import { MessageView } from "./Message.tsx";
import { flags } from "./flags.ts";
import { Message } from "sdk";

export const Debug = () => {
	return (
		<div style="padding:8px">
			<h3>experiments</h3>
			<InviteView />
			<Show when={flags.has("message_search")}>
				<Search />
			</Show>
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
	const invite = api.invites.fetch(inviteCode);

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
