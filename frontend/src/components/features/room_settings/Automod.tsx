import fuzzysort from "fuzzysort";
import type { Room, AutomodRule as SdkAutomodRule } from "sdk";
import {
	createEffect,
	createMemo,
	createResource,
	createSignal,
	For,
	Show,
	type VoidProps,
} from "solid-js";
import { createStore } from "solid-js/store";
import { uuidv7 } from "uuidv7";
import { useApi, useChannels } from "@/api";
import { Savebar } from "@/atoms/Savebar.tsx";
import { Search } from "@/atoms/Search";
import { AutomodRule } from "@/components/features/automod_old/AutomodRule";
import { useCurrentUser } from "@/contexts/currentUser.tsx";
import { useModals } from "@/contexts/modal";
import { usePermissions } from "@/hooks/usePermissions";
import { AutomodRule2 } from "../automod/AutomodRule";
import { AutomodProvider, useAutomod } from "../automod/context";

// clean = not touched, data is straight from the server
// draft = not yet created
// edited = rule exists on server, has unsaved changes
export type RuleState = "clean" | "draft" | "edited";

export type UiAutomodRule = SdkAutomodRule & { state: RuleState };

export function Automod(props: VoidProps<{ room: Room }>) {
	return (
		<AutomodProvider room_id={props.room.id}>
			<AutomodInner room={props.room} />
		</AutomodProvider>
	);
}

export function AutomodInner(props: VoidProps<{ room: Room }>) {
	const api2 = useApi();
	const channels2 = useChannels();
	const [, modalCtl] = useModals();
	const am = useAutomod();

	const roomChannels = createMemo(() => {
		return [...channels2.cache.values()].filter(
			(c) => c.room_id === props.room.id,
		);
	});

	const [rules, { refetch }] = createResource(async () => {
		const { data } = await api2.client.http.GET(
			"/api/v1/room/{room_id}/automod/rule",
			{ params: { path: { room_id: props.room.id } } },
		);
		return (data ?? []) as UiAutomodRule[];
	});

	const [search, setSearch] = createSignal("");
	const [draftRules, setDraftRules] = createStore<UiAutomodRule[]>([]);

	const filteredRules = () => {
		const query = search();
		const allRules = [...(rules() || []), ...draftRules];
		if (!query) return allRules;
		const results = fuzzysort.go(query, allRules, {
			key: "name",
			threshold: -10000,
		});
		return results.map((r) => r.obj);
	};

	const test = () => {
		// TODO: modal(?) to test automod rules
	};

	return (
		<div class="room-settings-automod">
			<h2>automod</h2>
			<header class="header">
				<Search placeholder="search" onInput={(s) => setSearch(s)} />
				<button type="button" class="button big" onClick={test}>
					test
				</button>
				<button type="button" class="button primary big" onClick={am.create}>
					create
				</button>
			</header>
			<For each={am.rules}>{(draft) => <AutomodRule2 draft={draft} />}</For>
			<button class="automod-rule create" onClick={am.create}>
				+ create rule
			</button>
			<Savebar onSave={am.save} onCancel={am.reset} show={am.dirty} />
		</div>
	);
}
