export function ServerInfo() {
	return (
		<>
			<h2>Info</h2>
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
