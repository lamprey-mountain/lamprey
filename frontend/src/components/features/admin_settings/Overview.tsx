import { createResource, For, Show } from "solid-js";
import type { ServerInfo } from "ts-sdk";
import { useApi } from "@/api";
import { CodeBlock } from "@/atoms/Markdown";
import { Copyable2 } from "@/utils/general";

export function Overview() {
	const api = useApi();

	const purgeCache = (target: string) => {
		api.client.http.POST("/api/v1/admin/purge-cache", {
			body: { targets: [target as any] },
		});
	};

	const gc = (target: string, mode: "Dry" | "Mark" | "Sweep") => {
		api.client.http.POST("/api/v1/admin/collect-garbage", {
			body: { targets: [target as any], async: false, mode },
		});
	};

	const gcDry = (target: string) => gc(target, "Dry");
	const gcMark = (target: string) => gc(target, "Mark");
	const gcSweep = (target: string) => gc(target, "Sweep");

	const [serverInfo] = createResource(async () => {
		const { data } = await api.client.http.GET("/api/v1/server/@self");
		return (data ?? null) as ServerInfo | null;
	});

	return (
		<div style="overflow: hidden;max-width: 100%;padding:4px;">
			<h2>Info</h2>
			<Show when={serverInfo()} fallback="loading...">
				{(info) => <Info info={info()} />}
			</Show>
			<h3 class="dim" style="margin-top:8px">
				Garbage collect
			</h3>
			<ul class="admin-tasks">
				<For
					each={["Media", "Messages", "Session", "AuditLog", "RoomAnalytics"]}
				>
					{(i) => (
						<li>
							<div class="name">{i}</div>
							<button type="button" class="button" onClick={[gcDry, i]}>
								dry
							</button>
							<button type="button" class="button" onClick={[gcMark, i]}>
								mark
							</button>
							<button type="button" class="button" onClick={[gcSweep, i]}>
								sweep
							</button>
						</li>
					)}
				</For>
			</ul>
			<h3 class="dim" style="margin-top:8px">
				Purge caches
			</h3>
			<ul class="admin-tasks">
				<For
					each={[
						"Channels",
						"Embeds",
						"Permissions",
						"Rooms",
						"Sessions",
						"Users",
					]}
				>
					{(i) => (
						<li>
							<div class="name">{i}</div>
							<button type="button" class="button" onClick={[purgeCache, i]}>
								purge
							</button>
						</li>
					)}
				</For>
			</ul>
			<br />
			<br />
			<ul>
				<li>stats (uptime, memory, etc)</li>
				<li>metrics</li>
				<li>
					cron jobs(?) (name, schedule, nest/last time, number of times
					executed)
				</li>
			</ul>
			<h2>Configuration</h2>
			<p>dump config.toml here</p>
			<p>
				configure auth sources ("login with xyz" idps). table of name, type
				(currently always oauth2), enabled, updated, created. or maye not, this
				could be done in config.toml
			</p>
			<h2>System Notices</h2>
			<p>
				copy forgejo. list notices in a table, id, type, option to select and
				delete.
			</p>
		</div>
	);
}

export function Info(props: { info: ServerInfo }) {
	return (
		<div style="display:flex;flex-direction:column">
			<h3 class="dim" style="margin-top:8px">
				urls
			</h3>
			<div style="overflow-x: auto">
				<table>
					<tbody>
						<tr>
							<td> api_url:</td>
							<td>
								<Copyable2 name="url">{props.info.api_url}</Copyable2>
							</td>
						</tr>
						<tr>
							<td> cdn_url:</td>
							<td>
								<Copyable2 name="url">{props.info.cdn_url}</Copyable2>
							</td>
						</tr>
						<tr>
							<td>html_url:</td>
							<td>
								<Copyable2 name="url">{props.info.html_url}</Copyable2>
							</td>
						</tr>
					</tbody>
				</table>
			</div>
			<h3 class="dim" style="margin-top:8px">
				features
			</h3>
			<div style="overflow-x: auto">
				<table>
					<thead>
						<tr>
							<td>feature</td>
							<td>configuration</td>
						</tr>
					</thead>
					<tbody>
						<For each={Object.entries(props.info.features)}>
							{([key, value]) => (
								<tr>
									<td>
										<b>{key}</b>
									</td>
									<td>
										<pre>{JSON.stringify(value, null, 2)}</pre>
									</td>
								</tr>
							)}
						</For>
					</tbody>
				</table>
			</div>

			<h3 class="dim" style="margin-top:8px">
				version
			</h3>
			<details>
				<summary>
					<b>{props.info.version.implementation}</b>{" "}
					{props.info.version.version}{" "}
					<span class="dim">(click for more info)</span>
				</summary>
				<CodeBlock
					text={JSON.stringify(props.info.version, null, 2)}
					lang="json"
				/>
			</details>
		</div>
	);
}
