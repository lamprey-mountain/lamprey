import {
	createEffect,
	createResource,
	createSignal,
	For,
	onCleanup,
	Show,
	type VoidProps,
} from "solid-js";
import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import type { Room } from "sdk";
import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import { Time } from "../Time.tsx";
import { usePermissions } from "../hooks/usePermissions.ts";
import {
	type AutomodRule,
	type AutomodRuleCreate,
	getTimestampFromUUID,
} from "sdk";
import { useModals } from "../contexts/modal";

export function Automod(props: VoidProps<{ room: Room }>) {
	const ctx = useCtx();
	const api = useApi();
	const [, modalCtl] = useModals();

	const [rules, { refetch }] = createResource(async () => {
		const { data } = await api.client.http.GET(
			"/api/v1/room/{room_id}/automod/rule",
			{ params: { path: { room_id: props.room.id } } },
		);
		return data;
	});

	const removeRule = (rule_id: string) => () => {
		modalCtl.confirm(
			"Are you sure you want to delete this rule?",
			(conf) => {
				if (!conf) return;
				api.client.http.DELETE(
					"/api/v1/room/{room_id}/automod/rule/{rule_id}",
					{ params: { path: { room_id: props.room.id, rule_id } } },
				).then(() => {
					refetch();
				});
			},
		);
	};

	const fetchMore = () => {
		// TODO: make this work
	};

	const [bottom, setBottom] = createSignal<Element | undefined>();

	createIntersectionObserver(() => bottom() ? [bottom()!] : [], (entries) => {
		for (const entry of entries) {
			if (entry.isIntersecting) fetchMore();
		}
	});

	const [search, setSearch] = createSignal("");
	// TODO: use fuzzysort here

	const create = () => {
		// TODO: show a automod rule "skeleton" ui, make user enter in name, triggers, actions, etc
	};

	const user_id = () => api.users.cache.get("@self")?.id;
	const perms = usePermissions(user_id, () => props.room.id, () => undefined);

	return (
		<div class="room-settings-integrations">
			<h2>automod</h2>
			<Show when={perms.has("RoomManage")}>
				<header class="applications-header">
					<input
						type="search"
						placeholder="search"
						aria-label="search"
						onInput={(e) => setSearch(e.target.value)}
					/>
					<button type="button" class="primary big" onClick={create}>
						create
					</button>
				</header>
			</Show>
			<Show when={rules()}>
				<ul>
					<For
						each={rules()!.filter((i) =>
							i.name.toLowerCase().includes(search().toLowerCase())
						)}
					>
						{(i) => {
							const [name, setName] = createSignal(i.name);
							const [enabled, setEnabled] = createSignal(i.enabled);

							createEffect(() => {
								setName(i.name);
								setEnabled(i.enabled);
							});

							const updateRule = async () => {
								await api.client.http.PATCH(
									"/api/v1/room/{room_id}/automod/rule/{rule_id}",
									{
										params: { path: { room_id: props.room.id, rule_id: i.id } },
										body: {
											name: name(),
											enabled: enabled(),
										},
									},
								);
							};

							return (
								<li>
									<details>
										<summary>
											<div style="display: flex; align-items: top; gap: 8px;">
												<div>
													<h3 class="name">{name()}</h3>
													<div>
														created on{" "}
														<Time date={getTimestampFromUUID(i.id)} />
													</div>
												</div>
											</div>
										</summary>
										<div class="info">
											<div style="display: flex;flex: 1; gap: 8px">
												<div style="flex: 1;">
													<h3 class="dim">name</h3>
													<input
														type="text"
														value={name()}
														onInput={(e) => setName(e.target.value)}
														onBlur={updateRule}
													/>
												</div>
												<div style="flex: 1;">
													{/* TODO: use fancy checkbox here */}
													<h3 class="dim">enabled</h3>
													<input
														type="checkbox"
														checked={enabled()}
														onChange={(e) => {
															setEnabled(e.target.checked);
															updateRule();
														}}
													/>
												</div>
											</div>
											{/* TODO: configure rule triggers */}
											{/* TODO: configure rule actions */}
											{/* TODO: configure rule exceptions */}
											<div style="margin-top: 8px; display: flex; gap: 8px">
												<button
													onClick={removeRule(i.id)}
													class="danger"
												>
													delete
												</button>
											</div>
										</div>
									</details>
								</li>
							);
						}}
					</For>
				</ul>
				<div ref={setBottom}></div>
			</Show>
		</div>
	);
}
