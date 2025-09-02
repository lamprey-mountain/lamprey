import { createResource, For, Show, type VoidProps } from "solid-js";
import { useApi } from "../api.tsx";
import { getTimestampFromUUID, type User } from "sdk";
import { getNotableChanges } from "../audit-log-util.ts";

export function AuditLog(props: VoidProps<{ user: User }>) {
	const api = useApi();

	const [log] = createResource(async () => {
		const { data } = await api.client.http.GET(
			"/api/v1/user/{user_id}/audit-logs",
			{
				params: { path: { user_id: "@self" } },
			},
		);
		return data;
	});

	const dateFmt = new Intl.DateTimeFormat("en-US", {
		dateStyle: "short",
		timeStyle: "short",
		hour12: false,
	});

	return (
		<>
			<h2>audit log</h2>
			<br />
			<Show when={log()}>
				<ul class="room-settings-audit-log">
					<For each={log()!.items}>
						{(entry) => {
							const ts = () => getTimestampFromUUID(entry.id);
							return (
								<li data-id={entry.id}>
									<div class="info">
										<h3>{entry.type}</h3>
									</div>
									<ul>
										<li>
											<em class="light">caused by:</em> you
										</li>
										<li>
											<em class="light">caused at:</em>{" "}
											<time datetime={ts().toISOString()}>
												{dateFmt.format(ts())}
											</time>
										</li>
										<Show when={entry.reason}>
											<li>
												<em class="light">reason:</em> {entry.reason}
											</li>
										</Show>
									</ul>
									<br />
									<h3>changes</h3>
									<ul>
										<For each={getNotableChanges(entry as any)}>
											{(c) => {
												return (
													<li>
														<em class="light">{c.key}:</em>{" "}
														{JSON.stringify(c.old) ?? "[null]"}{" "}
														<em class="light">-&gt;</em>{" "}
														{JSON.stringify(c.new) ?? "[null]"}
													</li>
												);
											}}
										</For>
									</ul>
									<br />
									<details>
										<summary>json</summary>
										<pre>{JSON.stringify(entry, null, 4)}</pre>
									</details>
								</li>
							);
						}}
					</For>
				</ul>
			</Show>
		</>
	);
}
