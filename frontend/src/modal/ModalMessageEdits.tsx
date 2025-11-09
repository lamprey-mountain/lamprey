import { diffChars } from "diff";
import { createResource, For } from "solid-js";
import { useApi } from "../api";
import { MessageView } from "../Message";
import { Modal } from "./mod";

export const ModalMessageEdits = (
	props: { thread_id: string; message_id: string },
) => {
	// FIXME: pagination
	const api = useApi();
	const [edits] = createResource(
		{ channel_id: props.thread_id, message_id: props.message_id },
		async (path) => {
			const { data } = await api.client.http.GET(
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
							const pages = diffChars(prev.content ?? "", i.content ?? "");
							const content = pages.map((i) => {
								if (i.added) return `<ins>${i.value}</ins>`;
								if (i.removed) return `<del>${i.value}</del>`;
								return i.value;
							}).join("");
							return (
								<li>
									<MessageView message={{ ...i, content }} separate />
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
