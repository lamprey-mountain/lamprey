import { type VoidProps } from "solid-js";
import { type User } from "sdk";
import { Checkbox } from "../icons";
import { useCtx } from "../context.ts";
import { Dropdown } from "../Dropdown.tsx";
import { CheckboxOption } from "../atoms/CheckboxOption";

export function Chat(props: VoidProps<{ user: User }>) {
	const ctx = useCtx();
	const { t } = useCtx();

	const toggle = (setting: string) => () => {
		const c = ctx.preferences();
		ctx.setPreferences({
			...c,
			frontend: {
				...c.frontend,
				[setting]: c.frontend[setting] === "yes" ? "no" : "yes",
			},
		});
	};

	return (
		<div class="user-settings-chat">
			<h2>{t("user_settings.chat")}</h2>
			<br />
			<div class="options">
				<h3 class="dim" style="margin-top:0">{t("user_settings.media")}</h3>
				<CheckboxOption
					id={`user-${props.user.id}-preview-attachments`}
					checked={ctx.preferences().frontend["preview_attachments"] === "yes"}
					onChange={() => toggle("preview_attachments")()}
					seed={`user-${props.user.id}-preview-attachments`}
				>
					<Checkbox
						checked={ctx.preferences().frontend["preview_attachments"] ===
							"yes"}
						seed={`user-${props.user.id}-preview-attachments`}
					/>
					<span>{t("user_settings.preview_attachments")}</span>
				</CheckboxOption>
				<CheckboxOption
					id={`user-${props.user.id}-preview-attachments-descriptions`}
					checked={ctx.preferences()
						.frontend["preview_attachments_descriptions"] === "yes"}
					onChange={() => toggle("preview_attachments_descriptions")()}
					seed={`user-${props.user.id}-preview-attachments-descriptions`}
				>
					<Checkbox
						checked={ctx.preferences()
							.frontend["preview_attachments_descriptions"] === "yes"}
						seed={`user-${props.user.id}-preview-attachments-descriptions`}
					/>
					<span>{t("user_settings.preview_attachments_descriptions")}</span>
				</CheckboxOption>
				<CheckboxOption
					id={`user-${props.user.id}-link-previews`}
					checked={ctx.preferences().frontend["link_previews"] === "yes"}
					onChange={() => toggle("link_previews")()}
					seed={`user-${props.user.id}-link-previews`}
				>
					<Checkbox
						checked={ctx.preferences().frontend["link_previews"] === "yes"}
						seed={`user-${props.user.id}-link-previews`}
					/>
					<span>{t("user_settings.link_previews")}</span>
				</CheckboxOption>
				<h3 class="dim">{t("user_settings.input")}</h3>
				<CheckboxOption
					id={`user-${props.user.id}-show-send-button`}
					checked={ctx.preferences().frontend["show_send_button"] === "yes"}
					onChange={() => toggle("show_send_button")()}
					seed={`user-${props.user.id}-show-send-button`}
				>
					<Checkbox
						checked={ctx.preferences().frontend["show_send_button"] === "yes"}
						seed={`user-${props.user.id}-show-send-button`}
					/>
					<span>{t("user_settings.show_send_button")}</span>
				</CheckboxOption>
				<CheckboxOption
					id={`user-${props.user.id}-typing-indicators`}
					checked={ctx.preferences().frontend["typing_indicators"] === "yes"}
					onChange={() => toggle("typing_indicators")()}
					seed={`user-${props.user.id}-typing-indicators`}
				>
					<Checkbox
						checked={ctx.preferences().frontend["typing_indicators"] === "yes"}
						seed={`user-${props.user.id}-typing-indicators`}
					/>
					<span>{t("user_settings.typing_indicators")}</span>
				</CheckboxOption>
				<h3 class="dim">{t("user_settings.content")}</h3>
				<div class="option apart">
					<div>
						<div>{t("user_settings.show_spoilers")}</div>
						<div class="dim">
							{t("user_settings.show_spoilers_description")}
						</div>
					</div>
					<Dropdown
						selected={ctx.preferences().frontend["spoilers"] || "click"}
						onSelect={(value) => {
							if (value) {
								const c = ctx.preferences();
								ctx.setPreferences({
									...c,
									frontend: {
										...c.frontend,
										spoilers: value,
									},
								});
							}
						}}
						options={[
							{ item: "click", label: t("user_settings.spoilers_click") },
							{ item: "hover", label: t("user_settings.spoilers_hover") },
							{ item: "always", label: t("user_settings.spoilers_always") },
						]}
					/>
				</div>
				<h3 class="dim">{t("user_settings.threads_sidebar")}</h3>
				<CheckboxOption
					id={`user-${props.user.id}-threads-sidebar-text`}
					checked={ctx.preferences().frontend["threads_sidebar_text"] === "yes"}
					onChange={toggle("threads_sidebar_text")}
					seed={`user-${props.user.id}-threads-sidebar-text`}
				>
					<Checkbox
						checked={ctx.preferences().frontend["threads_sidebar_text"] ===
							"yes"}
						seed={`user-${props.user.id}-threads-sidebar-text`}
					/>
					<span>{t("user_settings.threads_sidebar_text")}</span>
				</CheckboxOption>
				<CheckboxOption
					id={`user-${props.user.id}-threads-sidebar-document`}
					checked={ctx.preferences().frontend["threads_sidebar_document"] ===
						"yes"}
					onChange={toggle("threads_sidebar_document")}
					seed={`user-${props.user.id}-threads-sidebar-document`}
				>
					<Checkbox
						checked={ctx.preferences().frontend["threads_sidebar_document"] ===
							"yes"}
						seed={`user-${props.user.id}-threads-sidebar-document`}
					/>
					<span>{t("user_settings.threads_sidebar_document")}</span>
				</CheckboxOption>
				<CheckboxOption
					id={`user-${props.user.id}-threads-sidebar-forum`}
					checked={ctx.preferences().frontend["threads_sidebar_forum"] ===
						"yes"}
					onChange={toggle("threads_sidebar_forum")}
					seed={`user-${props.user.id}-threads-sidebar-forum`}
				>
					<Checkbox
						checked={ctx.preferences().frontend["threads_sidebar_forum"] ===
							"yes"}
						seed={`user-${props.user.id}-threads-sidebar-forum`}
					/>
					<span>{t("user_settings.threads_sidebar_forum")}</span>
				</CheckboxOption>
			</div>
		</div>
	);
}
