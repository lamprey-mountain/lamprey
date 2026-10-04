import {
	createEffect,
	createMemo,
	createResource,
	createSelector,
	createSignal,
	For,
	type JSX,
	Match,
	onCleanup,
	Show,
	Switch,
} from "solid-js";
import type { EvalInputSummary } from "ts-sdk";
import { useApi } from "@/api";
import { Duration } from "@/atoms/Duration";
import { Icon } from "@/atoms/Icon";
import { Time } from "@/atoms/Time";
import { usePanes } from "@/components/panes/context";
import { HTTP_STATUS_TEXT } from "@/lib/http";
import { getDate } from "@/utils/general";
import { icQuestion } from "@/utils/icons";
import { type ScriptPane, useScript } from "./context";
import { LazyCodeEditor } from "./LazyEditor";

export const ScriptCode = (props: {
	// FIXME: correct pane type
	// pane: Extract<ScriptPaneT, { type: "leaf", data: { type: "script_code" } }>;
	pane: Extract<ScriptPane, { type: "leaf" }>;
	setHeaderExtra: (el: JSX.Element) => void;
}) => {
	const api = useApi();
	const script = () => api.scripts.get(props.pane.data.script_id);

	// TODO: use api.scripts.use

	// TODO: move these to context.tsx
	const [editedSource, setEditedSource] = createSignal<string>("");
	const [saving, setSaving] = createSignal(false);

	// FIXME: show when code is dirty
	// FIXME: handle saving
	// TODO(?): keybind ctrl+s to save/commit/deploy code? though it could be confusing to people since its not really the same as saving?

	const hasEdits = () => {
		// const orig = source() ?? "";
		// const curr = editedSource();
		// return curr !== "" && curr !== orig;
		return false;
	};

	const handleSave = async () => {
		// const scr = script();
		// if (!scr) return;
		// setSaving(true);
		// try {
		// 	await scriptsService.uploadAndSaveContent(
		// 		scr.channel_id,
		// 		scr.id,
		// 		editedSource(),
		// 	);
		// 	mutate(editedSource());
		// } catch (err) {
		// 	console.error("Failed to save script:", err);
		// } finally {
		// 	setSaving(false);
		// }
	};

	createEffect(() => {
		props.setHeaderExtra(
			<Show when={hasEdits()}>
				<button
					type="button"
					class="pane-header-save button primary"
					onClick={handleSave}
					disabled={saving()}
				>
					{saving() ? "Saving..." : "Save Edits"}
				</button>
			</Show>,
		);
	});

	onCleanup(() => {
		props.setHeaderExtra(null);
	});

	return (
		<div class="script-code-container">
			<div class="editor-wrapper">
				<Show when={script()}>
					{(script) => (
						// PERF: lazy load LazyCodeEditor immediately instead of waiting for script()
						<LazyCodeEditor script={script()} onChange={setEditedSource} />
					)}
				</Show>
			</div>
		</div>
	);
};

export const ScriptInputs = (props: {
	pane: Extract<ScriptPane, { type: "leaf" }>;
}) => {
	const s = useScript();
	const panes = usePanes<ScriptPane>();
	const api = useApi();
	const scriptId = () => props.pane.data.script_id;

	const script = api.scripts.use(() => `${s.channel_id}:${scriptId()}`);

	createEffect(() => {
		api.scriptRuns.list(s.channel_id, scriptId());
	});

	// show newest evals first
	const runs = createMemo(() =>
		[...api.scriptRuns.cache.values()]
			.toSorted(
				(a, b) =>
					getDate(b.created_at).valueOf() - getDate(a.created_at).valueOf(),
			)
			.slice(0, 32),
	);

	const trigger = async (inputId: string) => {
		await api.scriptRuns.trigger(s.channel_id, scriptId(), {
			async: true,
			exclusive: false,
			trigger_id: inputId,
		});
	};

	const openLogs = (runId: string) => {
		const existingLogPane = panes.find(
			(p) => p.type === "leaf" && p.data.type === "run_logs",
		);
		if (existingLogPane) {
			panes.update(existingLogPane.id, {
				type: "leaf",
				data: {
					type: "run_logs",
					script_id: scriptId(),
					run_id: runId,
				},
			});
		} else {
			panes.split(
				props.pane.id,
				{
					type: "leaf",
					data: {
						type: "run_logs",
						script_id: props.pane.data.script_id,
						run_id: runId,
					},
				},
				"vertical",
			);
		}
	};

	return (
		<div class="script-inputs">
			<section>
				<h3>Inputs</h3>
				<div class="input-list">
					<For each={script()?.handlers}>
						{(input) => (
							<div class="script-input" data-input-type={input.type}>
								<Show when={input.type === "Manual"}>
									<button
										class="inner"
										type="button"
										onClick={[trigger, input.id]}
									>
										<div>{input.label}</div>
										<div class="dim">
											{input.type} {input.id}
										</div>
									</button>
								</Show>
								<Show when={input.type !== "Manual"}>
									<div class="inner">
										<div>{input.label}</div>
										<div class="dim">{input.id}</div>
									</div>
								</Show>
							</div>
						)}
					</For>
				</div>
			</section>
			<section>
				<h3>Recent Evals</h3>
				<ul class="eval-list">
					<For each={runs()}>
						{(run) => {
							function matchesInput<T extends EvalInputSummary["type"]>(
								ty: T,
							): (EvalInputSummary & { type: T }) | false {
								if (run.input.type === ty) {
									return run.input as EvalInputSummary & { type: T };
								} else {
									return false;
								}
							}

							return (
								<li>
									<div class="eval-item" onClick={[openLogs, run.id]}>
										<div class="eval-info">
											<span class="status" data-status={run.status}>
												{run.status}
											</span>
											<Time date={getDate(run.created_at)} />
										</div>
										<div style="display:flex">
											<Switch>
												<Match when={matchesInput("Extraction")}>
													Extraction
												</Match>
												<Match when={matchesInput("Http")}>
													{(input) => {
														const r = input().request;

														return (
															<>
																http
																{r.request_method}
																{r.request_url}
																{" -> "}
																{r.response_status}
																{HTTP_STATUS_TEXT[r.response_status]}
															</>
														);
													}}
												</Match>
												<Match when={matchesInput("Manual")}>
													{(input) => (
														<>
															{input().id}
															{input().user_id}
														</>
													)}
												</Match>
												<Match when={matchesInput("Event")}>
													{(input) => <>event {input().event.type}</>}
												</Match>
											</Switch>
											<div style="flex:1"></div>
											<button class="view-logs" type="button">
												View Logs
											</button>
										</div>
									</div>
								</li>
							);
						}}
					</For>
				</ul>
			</section>
		</div>
	);
};

export const ScriptPreview = () => {
	// needs backend support
	// would render http page for http endpoint, for example
	return "todo";
};

// TODO: use table instead of flex
export const RunLogs = (props: {
	pane: Extract<ScriptPane, { type: "leaf" }>;
	// pane: Extract<ScriptPane, { type: "run_logs" }>;
}) => {
	const s = useScript();
	const api = useApi();

	const scriptId = () => props.pane.data.script_id;
	const runId = () => props.pane.data.run_id;
	const channelId = () => s.channel_id;

	const [logResource] = createResource(
		() => [channelId(), scriptId(), runId()] as const,
		([c, sid, rid]) => api.scriptLogs.list(c, sid, rid),
	);

	const runInfo = api.scriptRuns.use(
		() => `${channelId()}:${scriptId()}:${runId()}`,
	);

	const [levelFilter, setLevelFilter] = createSignal<string>("all");
	const [expandedEntry, setExpandedEntry] = createSignal<number | null>(null);
	const isLevelFilterSelected = createSelector(levelFilter);

	const filteredLogs = () => {
		const filter = levelFilter();
		if (filter === "all") return api.scriptLogs.getLogsForRun(runId());
		return api.scriptLogs
			.getLogsForRun(runId())
			.filter((e) => e.level === filter);
	};

	const hasAttrs = (entry: { attributes?: Record<string, unknown> }) =>
		entry.attributes && Object.keys(entry.attributes).length > 0;

	const toggleExpand = (entryId: number) => {
		setExpandedEntry((prev) => (prev === entryId ? null : entryId));
	};

	const handleStop = async () => {
		await api.scriptRuns.stop(channelId(), scriptId(), runId());
	};

	const formatAttrsSummary = (attrs?: Record<string, unknown>) => {
		if (!attrs) return "";
		return Object.entries(attrs)
			.map(([key, val]) => {
				let valStr = String(val);
				if (valStr.length > 20) {
					valStr = valStr.substring(0, 17) + "...";
				}
				return `${key}=${valStr}`;
			})
			.join(" ");
	};

	const [isTimeRelative, setIsTimeRelative] = createSignal(false);
	const ts = createMemo(() => {
		const ts = runInfo()?.created_at;
		return ts ? getDate(ts) : new Date();
	});

	return (
		<div class="eval-logs">
			<Show when={logResource.loading}>
				<div>Loading logs...</div>
			</Show>
			<Show when={logResource.error}>
				<div>Error: {logResource.error}</div>
			</Show>
			<Show when={!logResource.loading && !logResource.error}>
				<Show when={runInfo()}>
					{(run) => (
						<div class="top">
							<span class="status" data-status={run().status}>
								{run().status}
							</span>
							<Show
								when={run().status === "Active" || run().status === "Creating"}
							>
								<button type="button" onClick={handleStop}>
									Stop
								</button>
							</Show>
						</div>
					)}
				</Show>
				<menu style="display:flex">
					<div class="log-filters">
						<For
							each={[
								{ id: "all", label: "All" },
								{ id: "Info", label: "Info" },
								{ id: "Warning", label: "Warning" },
								{ id: "Error", label: "Error" },
							]}
						>
							{(a) => (
								<button
									type="button"
									class="menu-button"
									onClick={[setLevelFilter, a.id]}
									aria-pressed={isLevelFilterSelected(a.id)}
								>
									<div class="inner">{a.label}</div>
								</button>
							)}
						</For>
					</div>
					<button
						type="button"
						class="menu-button time-button"
						onClick={() => setIsTimeRelative((a) => !a)}
					>
						{/* TODO: add a clock or timer icon */}
						<Icon src={icQuestion} />
						{isTimeRelative() ? "relative" : "absolute"}
					</button>
				</menu>
				<ul role="log">
					<For each={filteredLogs()}>
						{(entry) => {
							const entryTs = getDate(entry.created_at);

							return (
								<li
									classList={{ expanded: expandedEntry() === entry.id }}
									onclick={[toggleExpand, entry.id]}
									style="cursor: pointer"
								>
									<div class="main">
										<span class="time">
											{isTimeRelative() ? (
												<Time date={entryTs} />
											) : (
												<Duration ms={entryTs.valueOf() - ts().valueOf()} />
											)}
										</span>
										<span class="level" data-level={entry.level}>
											{entry.level}
										</span>
										<span class="content">{entry.content}</span>
										<Show when={hasAttrs(entry)}>
											<span class="attrs-summary">
												{formatAttrsSummary(entry.attributes)}
											</span>
										</Show>
									</div>
									<Show when={expandedEntry() === entry.id && hasAttrs(entry)}>
										<ul class="attrs expanded">
											<For each={Object.entries(entry.attributes ?? {})}>
												{([key, val]) => (
													<li>
														<span class="key">{key}</span>
														<span class="syn">=</span>
														<span class="val">{String(val)}</span>
													</li>
												)}
											</For>
										</ul>
									</Show>
								</li>
							);
						}}
					</For>
				</ul>
			</Show>
		</div>
	);
};
