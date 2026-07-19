import { diffChars } from "diff";
import type { Message, MessageVersion } from "sdk";
import { createMemo, createResource, For, Show } from "solid-js";
import { useApi, useMessages } from "@/api";
import { MessageView } from "@/components/features/chat/Message";
import { MessageToolbarProvider } from "@/components/features/chat/message-toolbar-context";
import {
	DEL_END,
	DEL_START,
	INS_END,
	INS_START,
	PUA_REGEX,
} from "@/utils/diff";
import { Modal } from "./mod";

export const ModalMessageEdits = (props: {
	channel_id: string;
	message_id: string;
}) => {
	// TODO: pagination
	// TODO: move to messages service
	const api2 = useApi();
	const messages = useMessages();
	const message = messages.use(
		() => props.channel_id,
		() => props.message_id,
	);

	const [edits] = createResource(
		{ channel_id: props.channel_id, message_id: props.message_id },
		async (path) => {
			const { data } = await api2.client.http.GET(
				"/api/v1/channel/{channel_id}/message/{message_id}/version",
				{
					params: {
						path,
						query: { limit: 100 },
					},
				},
			);
			return data!;
		},
	);

	// FIXME: incorrect openapi type
	const versions = () => (edits()?.items ?? []) as unknown as MessageVersion[];

	return (
		<Modal>
			<h3 style="margin: -8px 6px">edit history</h3>
			<Show when={message()}>
				{(message) => (
					<MessageToolbarProvider>
						<ul class="edit-history">
							<For each={versions()} fallback={"loading"}>
								{(version, idx) => {
									const prev = versions()[idx() - 1];

									const m = createMemo((): Message => {
										if (prev) {
											const prevContent =
												prev.type === "DefaultMarkdown"
													? (prev.content ?? "")
													: "";
											const versionContent =
												version.type === "DefaultMarkdown"
													? (version.content ?? "")
													: "";

											const changes = diffChars(prevContent, versionContent);
											const content = changes
												.map((i) => {
													const safeValue = i.value.replace(PUA_REGEX, "");
													if (i.added)
														return `${INS_START}${safeValue}${INS_END}`;
													if (i.removed)
														return `${DEL_START}${safeValue}${DEL_END}`;
													return safeValue;
												})
												.join("");
											return {
												...message(),
												latest_version: {
													...version,
													content,
												} as MessageVersion,
											};
										} else {
											return {
												...message(),
												latest_version: version as MessageVersion,
											};
										}
									});

									return (
										<li>
											<MessageView message={m()} separate diff />
										</li>
									);
								}}
							</For>
						</ul>
					</MessageToolbarProvider>
				)}
			</Show>
		</Modal>
	);
};
