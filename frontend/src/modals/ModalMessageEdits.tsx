import { diffChars } from "diff";
import { createResource, For } from "solid-js";
import { useApi2 } from "@/api";
import { MessageView } from "../components/features/chat/Message";
import { Modal } from "./mod";
import type { Message, MessageVersion } from "sdk";

export const ModalMessageEdits = (
	props: { thread_id: string; message_id: string },
) => {
	// FIXME: pagination
	const api2 = useApi2();
	const [edits] = createResource(
		{ channel_id: props.thread_id, message_id: props.message_id },
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

	diffChars;

	return (
		<Modal>
			<h3 style="margin: -8px 6px">edit history</h3>
			<ul class="edit-history">
				<For each={edits()?.items ?? []} fallback={"loading"}>
					{(i, x) => {
						const prev = edits()?.items[x() - 1];
						if (prev) {
							const prevVersion = prev.latest_version;
							const currVersion = i.latest_version;
							const prevContent = prevVersion.type === "DefaultMarkdown"
								? prevVersion.content ?? ""
								: "";
							const currContent = currVersion.type === "DefaultMarkdown"
								? currVersion.content ?? ""
								: "";
							const pages = diffChars(
								prevContent,
								currContent,
							);
							const content = pages.map((i) => {
								if (i.added) return `<ins>${i.value}</ins>`;
								if (i.removed) return `<del>${i.value}</del>`;
								return i.value;
							}).join("");
							const messageWithContent: Message = {
								...i,
								latest_version: {
									...currVersion,
									content,
								} as MessageVersion,
							};
							return (
								<li>
									<MessageView message={messageWithContent} separate />
								</li>
							);
						} else {
							return (
								<li>
									<MessageView message={i} separate />
								</li>
							);
						}
					}}
				</For>
			</ul>
		</Modal>
	);
};
