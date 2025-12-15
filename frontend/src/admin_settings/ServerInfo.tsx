import { For } from "solid-js";
import { useApi } from "../api";

export function ServerInfo() {
	const api = useApi();

	const purgeCache = (target: string) => {
		// TODO
	};

	const gcDry = (target: string) => {
		// TODO
	};

	const gcMark = (target: string) => {
		// TODO
	};

	const gcSweep = (target: string) => {
		// TODO
	};

	return (
		<>
			<h2>Info</h2>
			<h3 class="dim" style="margin-top:8px">Garbage collect</h3>
			<ul class="admin-tasks">
				<For
					each={[
						"Media",
						"Messages",
						"Session",
						"AuditLog",
						"RoomAnalytics",
					]}
				>
					{(i) => (
						<li>
							<div class="name">{i}</div>
							<button onClick={[gcDry, i]}>dry</button>
							<button onClick={[gcMark, i]}>mark</button>
							<button onClick={[gcSweep, i]}>sweep</button>
						</li>
					)}
				</For>
			</ul>
			<h3 class="dim" style="margin-top:8px">Purge caches</h3>
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
							<button onClick={[purgeCache, i]}>purge</button>
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
			<h2>Reports</h2>
			<p>copy report-to-mod system?</p>
			<h2>Configuration</h2>
			<p>dump config.toml here</p>
			<p>
				configure auth sources ("login with xyz" idps). table of name, type
				(currently always oauth2), enabled, updated, created. or maye not, this
				could be done in config.toml
			</p>
			<form>
				<label>
					provider name
					<input type="text" />
				</label>
				<br />
				<label>
					provider icon (optional)
					<input type="text" />
				</label>
				<br />
				<label>
					client id
					<input type="text" />
				</label>
				<br />
				<label>
					client secret
					<input type="text" />
				</label>
				<br />
				<label>
					enabled
					<input type="checkbox" />
				</label>
				<br />
				<label>
					scopes
					<input type="text" />
				</label>
			</form>
			<h2>System Notices</h2>
			<p>
				copy forgejo. list notices in a table, id, type, option to select and
				delete.
			</p>
		</>
	);
}
