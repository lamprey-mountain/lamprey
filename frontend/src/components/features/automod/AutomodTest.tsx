import { debounce } from "@solid-primitives/scheduled";
import {
	createResource,
	createSignal,
	For,
	Match,
	Show,
	Suspense,
	Switch,
} from "solid-js";
import type { AutomodAction, AutomodMatch, AutomodRule } from "ts-sdk";
import { useApi } from "@/api";
import { Dropdown } from "@/atoms/Dropdown";
import { Duration } from "@/atoms/Duration";
import { useAutomod } from "./context";

type AutomodTest = {
	rules: AutomodRule[];
	matches: AutomodMatch | null;
	actions: AutomodAction[];
};

export const AutomodTest = () => {
	const api = useApi();
	const am = useAutomod();

	const [text, setText] = createSignal("");
	const [debouncedText, setDebouncedText] = createSignal("");
	const [target, setTarget] = createSignal<"Content" | "Member">("Content");

	const debouncedSetText = debounce((value: string) => {
		setDebouncedText(value);
	}, 300);

	const [scan] = createResource(
		() => ({ text: debouncedText(), target: target() }),
		async ({ text, target }) => {
			if (!text) return null;
			const { data } = await api.client.http.POST(
				"/api/v1/room/{room_id}/automod/rule/test",
				{
					params: { path: { room_id: am.roomId } },
					body: {
						text,
						target,
					},
				},
			);
			// TODO: better error handling
			return data as unknown as AutomodTest;
		},
	);

	return (
		<div class="automod-test">
			<header class="header">
				<h2>rule tester</h2>
				<label style="display: block; margin: 8px 0">
					<h3 class="dim">Target</h3>
					<textarea
						class="textarea"
						placeholder="insert some text here"
						value={text()}
						onInput={(e) => {
							const val = e.currentTarget.value;
							setText(val);
							debouncedSetText(val);
						}}
					></textarea>
				</label>
				<label style="display: block; margin: 8px 0">
					<h3 class="dim">Text</h3>
					<Dropdown
						options={[
							{
								item: "Content",
								label: "content",
								view: (
									<div>
										<div>Content</div>
										<div class="dim">
											user submitted content: messages, thread titles, voice
											statuses, etc
										</div>
									</div>
								),
							},
							{
								item: "Member",
								label: "member",
								view: (
									<div>
										<div>Member</div>
										<div class="dim">
											text on member profiles: user names, bios, and nicknames
										</div>
									</div>
								),
							},
						]}
						selected={target()}
						onSelect={(val) => setTarget(val!)}
						required
						enableWheel={false}
					/>
				</label>
			</header>
			<div class="results">
				<Suspense fallback={<h3 class="section dim">loading...</h3>}>
					<h3 class="section dim">matched rules</h3>
					<For
						each={scan()?.rules ?? []}
						fallback={<div class="card">No rules matched!</div>}
					>
						{(a) => (
							<div class="card">
								{a.name}
								{/* TODO: click to jump to automod rule */}
							</div>
						)}
					</For>
					<Show when={scan()?.matches}>
						{(m) => (
							<>
								<h3 class="section dim">matched text</h3>
								<div class="card">
									{/* TODO: highlight slice of text that matched */}
									{/* TODO: show matcher pattern (eg. regex, keywords) */}
									<ul class="clauses">
										<For each={m().fragments ?? []}>
											{(i) => (
												<li>
													<span class="dim">match ({i.matcher}):</span> {i.text}
												</li>
											)}
										</For>
									</ul>
								</div>
							</>
						)}
					</Show>
					<Show when={scan()?.rules.length}>
						<h3 class="section dim">actions that would be executed</h3>
						<For each={scan()?.actions ?? []}>
							{(action) => {
								function matchesAction<T extends AutomodAction["type"]>(
									ty: T,
								): (AutomodAction & { type: T }) | false {
									if (action.type === ty) {
										return action as AutomodAction & { type: T };
									} else {
										return false;
									}
								}

								return (
									<div class="card">
										<ul class="clauses">
											<li>
												<span class="dim">Action type:</span> {action.type}
											</li>

											<Switch>
												<Match when={matchesAction("Block")}>
													{(action) => (
														<li>
															<span class="dim">With message:</span>{" "}
															{action().message}
														</li>
													)}
												</Match>
												<Match when={matchesAction("Timeout")}>
													{(action) => (
														<li>
															<span class="dim">With duration:</span>{" "}
															<Duration ms={action().duration} />
														</li>
													)}
												</Match>
												<Match when={matchesAction("SendAlert")}>
													{/* TODO: render actual channel name */}
													{(action) => (
														<li>
															<span class="dim">To channel:</span>{" "}
															{action().channel_id}
														</li>
													)}
												</Match>
											</Switch>
										</ul>
									</div>
								);
							}}
						</For>
					</Show>
				</Suspense>
			</div>
		</div>
	);
};
