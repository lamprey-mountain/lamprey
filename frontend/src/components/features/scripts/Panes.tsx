import {
	createResource,
	createSignal,
	For,
	type JSX,
	onCleanup,
	Show,
} from "solid-js";
import { useApi, useScriptRuns, useScripts } from "@/api";
import { Time } from "@/atoms/Time";
import { usePanes } from "@/components/panes/context";
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

	// createEffect(() => {
	// 	const doc = ydoc();
	// 	if (!doc) return;
	// 	const s = script();
	// 	if (!s) return;
	// 	const { channel_id, id: script_id } = s;

	// 	const onSync = ([sync]: [any, unknown]) => {
	// 		if (sync.type === "DocumentEdit") {
	// 			if (sync.channel_id !== channel_id) return;
	// 			if (sync.branch_id !== script_id) return;
	// 			const update = (
	// 				(sync.update as unknown) instanceof Uint8Array
	// 					? sync.update
	// 					: base64UrlDecode(sync.update)
	// 			) as Uint8Array;
	// 			Y.applyUpdate(doc, update, { key: "server" });
	// 		}
	// 	};

	// 	const unsub = api.events.on("sync", onSync);
	// 	onCleanup(unsub);
	// });

	// const hasEdits = () => {
	// 	const orig = source() ?? "";
	// 	const curr = editedSource();
	// 	return curr !== "" && curr !== orig;
	// };

	// const handleSave = async () => {
	// 	const scr = script();
	// 	if (!scr) return;
	// 	setSaving(true);
	// 	try {
	// 		await scriptsService.uploadAndSaveContent(
	// 			scr.channel_id,
	// 			scr.id,
	// 			editedSource(),
	// 		);
	// 		mutate(editedSource());
	// 	} catch (err) {
	// 		console.error("Failed to save script:", err);
	// 	} finally {
	// 		setSaving(false);
	// 	}
	// };

	// createEffect(() => {
	// 	props.setHeaderExtra(
	// 		<Show when={hasEdits()}>
	// 			<button
	// 				type="button"
	// 				class="pane-header-save button primary"
	// 				onClick={handleSave}
	// 				disabled={saving()}
	// 			>
	// 				{saving() ? "Saving..." : "Save Edits"}
	// 			</button>
	// 		</Show>,
	// 	);
	// });

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

	const [script] = createResource(
		() => `${s.channel_id}:${scriptId()}`,
		(id) => api.scripts.fetch(id),
	);

	const [runs, { refetch: refetchRuns }] = createResource(scriptId, (id) =>
		api.scriptRuns.list(s.channel_id, id),
	);

	const trigger = async (inputId: string) => {
		await api.scriptRuns.trigger(s.channel_id, scriptId(), {
			async: true,
			exclusive: false,
			trigger_id: inputId,
		});
		refetchRuns();
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
										onClick={() => trigger(input.id)}
									>
										<div>{input.label}</div>
										<div class="dim">{input.id}</div>
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
				<h3>Recent Runs</h3>
				<ul class="run-list">
					<For each={runs()?.items}>
						{(run) => (
							<li>
								<div class="run-item">
									<div class="run-info">
										<span class="status" data-status={run.status}>
											{run.status}
										</span>
										<Time date={new Date(run.created_at)} />
									</div>
									<menu>
										<button type="button" onClick={() => openLogs(run.id)}>
											Logs
										</button>
									</menu>
								</div>
							</li>
						)}
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

	const [runInfo] = createResource(runId, (rid) =>
		api.scriptRuns.fetch(`${channelId()}:${scriptId()}:${rid}`),
	);

	const [levelFilter, setLevelFilter] = createSignal<string>("all");
	const [expandedEntry, setExpandedEntry] = createSignal<number | null>(null);

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

	return (
		<div class="run-logs">
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
				<div class="log-filters">
					<button
						type="button"
						onClick={() => setLevelFilter("all")}
						aria-pressed={levelFilter() === "all"}
					>
						All
					</button>
					<button
						type="button"
						onClick={() => setLevelFilter("Info")}
						aria-pressed={levelFilter() === "Info"}
					>
						Info
					</button>
					<button
						type="button"
						onClick={() => setLevelFilter("Warning")}
						aria-pressed={levelFilter() === "Warning"}
					>
						Warning
					</button>
					<button
						type="button"
						onClick={() => setLevelFilter("Error")}
						aria-pressed={levelFilter() === "Error"}
					>
						Error
					</button>
				</div>
				<ul role="log">
					<For each={filteredLogs()}>
						{(entry) => (
							<li
								classList={{ expanded: expandedEntry() === entry.id }}
								onclick={() => toggleExpand(entry.id)}
								style="cursor: pointer"
							>
								<div class="main">
									<span class="time">
										<Time date={new Date(entry.created_at)} />
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
						)}
					</For>
				</ul>
			</Show>
		</div>
	);
};
