import fuzzysort from "fuzzysort";
import type { User } from "sdk";
import {
	createSelector,
	createSignal,
	For,
	Match,
	Show,
	Switch,
	type VoidProps,
} from "solid-js";
import { Resizable } from "@/atoms/Resizable";
import { Savebar } from "@/atoms/Savebar";
import { Search } from "@/atoms/Search";
import { Avatar } from "@/components/shared/User";
import {
	type ApplicationDraft,
	ApplicationsProvider,
	useApplications,
} from "../applications/context";
import { Oauth } from "../applications/Oauth";
import { Overview } from "../applications/Overview";
import { Sessions } from "../applications/Sessions";

export function Applications(_props: VoidProps<{ user: User }>) {
	return (
		<ApplicationsProvider>
			<ApplicationsInner />
		</ApplicationsProvider>
	);
}

export function ApplicationsInner() {
	const am = useApplications();

	const [search, setSearch] = createSignal("");

	const filteredApps = () => {
		const query = search();
		const allApps = am.apps;
		if (!query) return allApps;
		const results = fuzzysort.go(query, allApps, {
			key: (a) => (a.state === "create" ? a.create.name : a.data.name),
			threshold: -10000,
		});
		return results.map((r) => r.obj);
	};

	const [editingId, setEditingId] = createSignal<string | null>(null);

	const findDraft = (id: string) =>
		am.apps.find((a) =>
			a.state === "create" ? a.nonce === id : a.data.id === id,
		);

	const editDraft = (draft: ApplicationDraft) => {
		const id = draft.state === "create" ? draft.nonce : draft.data.id;
		if (editingId() === id) {
			setEditingId(null);
		} else {
			setEditingId(id);
		}
	};

	return (
		<div class="user-settings-applications">
			<div class="room-settings-roles">
				<div class="role-main">
					<h2>applications</h2>
					<header class="applications-header">
						<Search placeholder="search" onInput={setSearch} />
						<button type="button" class="button primary" onClick={am.create}>
							create
						</button>
					</header>
					<ul class="applications-list">
						<For each={filteredApps()}>
							{(draft) => {
								const appData =
									draft.state === "create"
										? {
												name: draft.create.name,
												avatar: null,
												description: null,
											}
										: {
												name: draft.data.name,
												avatar: draft.data.avatar,
												description: draft.data.description,
											};

								// TODO: deduplicate code
								const appWithAvatar = () => ({
									id: draft.state === "create" ? draft.nonce : draft.data.id,
									name: appData.name,
									avatar: appData.avatar ?? null,
									banner: null,
									description: null,
									bot: false,
									system: false,
									version_id: "",
									flags: 0,
									presence: { status: "Offline" as const, activities: [] },
									preferences: null,
								});

								return (
									<li
										onClick={() => editDraft(draft)}
										data-draft-state={draft.state}
									>
										<div class="info">
											<Avatar user={appWithAvatar()} pad={4} />
											<div style="display: flex; flex-direction:column;">
												<h3 class="name">{appData.name}</h3>
												<Show when={appData.description}>
													<div class="description">{appData.description}</div>
												</Show>
											</div>
										</div>
									</li>
								);
							}}
						</For>
					</ul>
					<Savebar show={am.dirty} onCancel={am.reset} onSave={am.save} />
				</div>
				<Show
					when={
						editingId() !== null &&
						(findDraft(editingId()!) as ApplicationDraft)
					}
				>
					{(draft) => (
						<Resizable
							storageKey="app-editor-width"
							initialWidth={400}
							minWidth={300}
							maxWidth={800}
							classList={{ "role-edit-resizable": true }}
						>
							<ApplicationEditor draft={draft()} />
						</Resizable>
					)}
				</Show>
			</div>
		</div>
	);
}

type ApplicationTab = "overview" | "oauth" | "sessions";
const ApplicationEditor = (props: { draft: ApplicationDraft }) => {
	const am = useApplications();
	const [activeTab, setActiveTab] = createSignal<ApplicationTab>("overview");

	const isActiveTab = createSelector(activeTab);

	return (
		<div class="application-edit">
			<div class="tabs">
				<For
					each={
						[
							{ id: "overview", label: "overview" },
							{ id: "oauth", label: "oauth" },
							{ id: "sessions", label: "sessions" },
						] as Array<{ id: ApplicationTab; label: string }>
					}
				>
					{(tab) => (
						<button
							type="button"
							class="button tab"
							classList={{ active: isActiveTab(tab.id) }}
							onClick={[setActiveTab, tab.id]}
						>
							{tab.label}
						</button>
					)}
				</For>
			</div>
			<div class="main">
				<Switch>
					<Match when={activeTab() === "overview"}>
						<Overview draft={props.draft} />
					</Match>
					<Match when={activeTab() === "oauth"}>
						<Oauth draft={props.draft} />
					</Match>
					<Match when={activeTab() === "sessions"}>
						<Sessions draft={props.draft} />
					</Match>
				</Switch>
			</div>
		</div>
	);
};
