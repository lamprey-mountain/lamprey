import { Show, type VoidProps } from "solid-js";
import { createUpload, type User } from "sdk";
import { useCtx } from "../context.ts";
import { useApi } from "../api.tsx";
import { getThumbFromId } from "../media/util.tsx";

export function Appearance(props: VoidProps<{ user: User }>) {
	const api = useApi();
	const ctx = useCtx();

	const toggle = (setting: string) => () => {
		const c = ctx.userConfig();
		ctx.setUserConfig({
			...c,
			frontend: {
				...c.frontend,
				[setting]: c.frontend[setting] === "yes" ? "no" : "yes",
			},
		});
	};

	return (
		<div class="user-settings-info">
			<h2>appearance</h2>
			<br />
			<label>
				<input
					type="checkbox"
					checked={ctx.userConfig().frontend["message_pfps"] === "yes"}
					onInput={toggle("message_pfps")}
				/>{" "}
				show pfps in messages (experimental)
			</label>
			<br />
			<label>
				<input
					type="checkbox"
					checked={ctx.userConfig().frontend["underline_links"] === "yes"}
					onInput={toggle("underline_links")}
				/>{" "}
				always underline links
			</label>
		</div>
	);
}
