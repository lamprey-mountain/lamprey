import type { VoidProps } from "solid-js";

export function Info(_props: VoidProps<{}>) {
	return (
		<>
			<h2>info</h2>
			<ul>
				<li>garbage collect media</li>
				<li>garbage collect cache</li>
				<li>garbage collect everything else</li>
				<li>stats (uptime, memory, etc)</li>
				<li>metrics</li>
				<li>
					cron jobs(?) (name, schedule, nest/last time, number of times
					executed)
				</li>
			</ul>
			<h2>users/rooms/threads/media/applications</h2>
			<ul>
				<li>name</li>
				<li>created</li>
				<li>topic (room/liread)</li>
				<li>size (media/room/user)</li>
				<li>registered (user)</li>
				<li>last used (user)</li>
				<li>suspended (user)</li>
				<li>metrics (room)</li>
			</ul>
			<ul>
				<li>search by name, email, id; sort by last used, created, id, name</li>
				<li>delete</li>
				<li>suspend</li>
				<li>create user</li>
				<li>view private stuff (email)</li>
				<li>change password, email, etc</li>
				<li>grant/revoke permissions (admin, can invite)</li>
				<li>force register (without invite)</li>
			</ul>
			<h2>reports</h2>
			<p>copy report-to-mod system?</p>
			<h2>configuration</h2>
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
			<h2>system notices/audit log</h2>
			<p>
				copy forgejo. list notices in a table, id, type, option to select and
				delete. maybe combine with audit log?
			</p>
			<ul>
				<li>user/room/thread/etc create/update/delete</li>
				<li>warnings and errors</li>
			</ul>
		</>
	);
}
