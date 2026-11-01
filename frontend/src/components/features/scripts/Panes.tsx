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
import type { EvalInputSummary, EvalLogEntry, ServerInfo } from "ts-sdk";

type DisplayLogEntry = Omit<EvalLogEntry, "level"> & {
	level: EvalLogEntry["level"] | "REQUEST" | "RESPONSE";
};

import { useApi } from "@/api";
import { Duration } from "@/atoms/Duration";
import { Icon } from "@/atoms/Icon";
import { Time } from "@/atoms/Time";
import { usePanes } from "@/components/panes/context";
import { HTTP_STATUS_TEXT } from "@/lib/http";
import { getDate } from "@/utils/general";
import { icExternal1, icQuestion } from "@/utils/icons";
import { type ScriptPane, useScript } from "./context";
import { LazyCodeEditor } from "./LazyEditor";

const httpStatusClass = (status: number) => {
	if (status >= 500) return "error";
	if (status >= 400) return "warning";
	if (status >= 300) return "redirect";
	if (status >= 200) return "success";
	return "unknown";
};

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

	const [serverInfo] = createResource(async () => {
		const { data } = await api.client.http.GET("/api/v1/server/@self");
		return (data ?? null) as ServerInfo | null;
	});

	const httpUrl = createMemo(() => {
		const info = serverInfo();
		const suffix = info?.features.redex?.suffix?.replace(/^\.+/, "");
		const shortname = script()?.shortname;
		if (!info || !suffix || !shortname) return;

		const { protocol } = new URL(info.api_url);
		return `${protocol}//${shortname}.${suffix}`;
	});

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
	const runIds = createMemo(() => runs().map((run) => run.id));
	const [activeRunId, setActiveRunId] = createSignal<string | null>(null);
	const tabbableRunId = () => {
		const ids = runIds();
		const active = activeRunId();
		return active && ids.includes(active) ? active : ids[0];
	};

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

	const handleRunKeyDown = (
		e: KeyboardEvent & { currentTarget: HTMLTableRowElement },
		runId: string,
	) => {
		if (e.key === "Enter" || e.key === " ") {
			e.preventDefault();
			openLogs(runId);
			return;
		}

		if (e.key !== "ArrowUp" && e.key !== "ArrowDown") return;

		const rows = Array.from(
			e.currentTarget
				.closest("table")
				?.querySelectorAll<HTMLTableRowElement>("tbody tr") ?? [],
		);
		const current = rows.indexOf(e.currentTarget);
		const next = rows[current + (e.key === "ArrowDown" ? 1 : -1)];
		if (!next) return;

		e.preventDefault();
		next.focus();
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
								<Show when={input.type === "Http"}>
									<Show
										when={httpUrl()}
										fallback={
											<div class="inner">
												<div>{input.label}</div>
												<div class="dim">{input.id}</div>
											</div>
										}
									>
										{(url) => (
											<a
												class="inner"
												href={url()}
												target="_blank"
												rel="noreferrer"
											>
												<div class="label">
													<span>{input.label}</span>
													<Icon
														class="external-icon"
														src={icExternal1}
														color="currentColor"
													/>
												</div>
												<div class="dim">{url()}</div>
											</a>
										)}
									</Show>
								</Show>
								<Show when={input.type === "Event"}>
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
				<table class="eval-list">
					<tbody>
						<For each={runIds()}>
							{(runId) => {
								const run = () => api.scriptRuns.cache.get(runId);
								const httpRequest = () => {
									const current = run();
									if (current?.input.type === "Http")
										return current.input.request;
								};

								function matchesInput<T extends EvalInputSummary["type"]>(
									ty: T,
								): (EvalInputSummary & { type: T }) | false {
									const current = run();
									if (current?.input.type === ty) {
										return current.input as EvalInputSummary & { type: T };
									} else {
										return false;
									}
								}

								return (
									<tr
										class="eval-item"
										tabIndex={tabbableRunId() === runId ? 0 : -1}
										onFocus={() => setActiveRunId(runId)}
										onClick={() => openLogs(runId)}
										onKeyDown={(e) => handleRunKeyDown(e, runId)}
									>
										<td class="status-cell">
											<span class="status-dot" data-status={run()?.status} />
										</td>
										<td class="input-cell">
											<Switch>
												<Match when={matchesInput("Extraction")}>
													Extraction
												</Match>
												<Match when={matchesInput("Http")}>
													{(input) => (
														<>
															<span class="http-method">
																{input().request.request_method}
															</span>{" "}
															<span class="http-url">
																{input().request.request_url}
															</span>
														</>
													)}
												</Match>
												<Match when={matchesInput("Manual")}>
													{(input) => `${input().id} ${input().user_id}`}
												</Match>
												<Match when={matchesInput("Event")}>
													{(input) => `event ${input().event.type}`}
												</Match>
											</Switch>
										</td>
										<td class="response-cell">
											<Show when={httpRequest()}>
												{(request) => (
													<Show when={request().response_status > 0}>
														<span
															class="http-status"
															data-code={httpStatusClass(
																request().response_status,
															)}
														>
															<span class="http-status-code">
																{request().response_status}
															</span>
															<Show
																when={
																	HTTP_STATUS_TEXT[request().response_status]
																}
																keyed
															>
																{(status) => (
																	<span class="http-status-reason">
																		{" "}
																		{status}
																	</span>
																)}
															</Show>
														</span>
													</Show>
												)}
											</Show>
										</td>
										<td class="time-cell">
											<Show when={run()}>
												{(current) => (
													<Time date={getDate(current().created_at)} />
												)}
											</Show>
										</td>
									</tr>
								);
							}}
						</For>
					</tbody>
				</table>
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

	const fakeHttpLogs = createMemo(() => {
		const run = runInfo();
		if (!run || run.input.type !== "Http") return;

		const r = run.input.request;
		const requestHeaders = Object.fromEntries(
			Object.entries(r.request_headers).map(([key, value]) => [
				key.toLowerCase(),
				value,
			]),
		);
		const responseHeaders = Object.fromEntries(
			Object.entries(r.response_headers).map(([key, value]) => [
				key.toLowerCase(),
				value,
			]),
		);

		let url: URL | undefined;
		try {
			url = new URL(r.request_url);
		} catch {
			url = undefined;
		}

		const authority =
			url?.host || requestHeaders.host || requestHeaders[":authority"];
		const scheme =
			url?.protocol.replace(":", "") || requestHeaders["x-forwarded-proto"];
		const path = url ? `${url.pathname}${url.search}` : r.request_url;

		const requestAttrs: Record<string, unknown> = {
			":method": r.request_method,
			...(scheme ? { ":scheme": scheme } : {}),
			...(authority ? { ":authority": authority } : {}),
			":path": path,
			...requestHeaders,
		};
		const requestEntry: DisplayLogEntry = {
			id: -1,
			created_at: run.created_at,
			level: "REQUEST",
			source: { type: "Runtime" },
			content: `${r.request_method} ${path}`,
			attributes: requestAttrs,
		};

		if (r.response_status <= 0) {
			return { request: requestEntry, response: undefined };
		}

		const responseAttrs: Record<string, unknown> = {
			":status": String(r.response_status),
			...responseHeaders,
		};
		const statusText = HTTP_STATUS_TEXT[r.response_status];
		const responseEntry: DisplayLogEntry = {
			id: -2,
			created_at: run.stopped_at ?? run.created_at,
			level: "RESPONSE",
			source: { type: "Runtime" },
			content: `${r.response_status}${statusText ? ` ${statusText}` : ""}`,
			attributes: responseAttrs,
		};

		return { request: requestEntry, response: responseEntry };
	});

	const filteredLogs = () => {
		const filter = levelFilter();
		const logs = api.scriptLogs.getLogsForRun(runId());
		const filtered =
			filter === "all" ? logs : logs.filter((e) => e.level === filter);
		const fake = fakeHttpLogs();
		if (!fake) return filtered;
		return [
			fake.request,
			...filtered,
			...(fake.response ? [fake.response] : []),
		];
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
