import fuzzysort from "fuzzysort";
import type { Room, AutomodRule as SdkAutomodRule } from "sdk";
import { createSignal, For, Show, type VoidProps } from "solid-js";
import { Resizable } from "@/atoms/Resizable";
import { Savebar } from "@/atoms/Savebar.tsx";
import { Search } from "@/atoms/Search";
import { AutomodRuleEditor } from "../automod/AutomodRule";
import { AutomodTest } from "../automod/AutomodTest";
import { AutomodProvider, useAutomod } from "../automod/context";

export function Automod(props: VoidProps<{ room: Room }>) {
	return (
		<AutomodProvider room_id={props.room.id}>
			<AutomodInner room={props.room} />
		</AutomodProvider>
	);
}

export function AutomodInner(props: VoidProps<{ room: Room }>) {
	const am = useAutomod();

	const [search, setSearch] = createSignal("");

	const filteredRules = () => {
		const query = search();
		const allRules = [...am.rules];
		if (!query) return allRules;
		const results = fuzzysort.go(query, allRules, {
			key: "name",
			threshold: -10000,
		});
		return results.map((r) => r.obj);
	};

	const [showTest, setShowTest] = createSignal(false);

	return (
		<div class="room-settings-automod" classList={{ testing: showTest() }}>
			<div class="automod-main">
				<h2>automod</h2>
				<header class="header">
					<Search placeholder="search" onInput={(s) => setSearch(s)} />
					<button
						type="button"
						class="button big"
						onClick={() => setShowTest((a) => !a)}
					>
						test
					</button>
					<button type="button" class="button primary big" onClick={am.create}>
						create
					</button>
				</header>
				<For each={filteredRules()}>
					{(draft) => <AutomodRuleEditor draft={draft} />}
				</For>
				<button class="automod-rule create" onClick={am.create}>
					+ create rule
				</button>
				<Savebar onSave={am.save} onCancel={am.reset} show={am.dirty} />
			</div>
			<Show when={showTest()}>
				<Resizable
					storageKey="automod-test-width"
					initialWidth={400}
					minWidth={300}
					maxWidth={800}
				>
					<AutomodTest />
				</Resizable>
			</Show>
		</div>
	);
}
