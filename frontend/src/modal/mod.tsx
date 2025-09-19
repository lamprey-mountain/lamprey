import {
	createEffect,
	createSignal,
	For,
	type ParentProps,
	Show,
} from "solid-js";
import { type Modal as ContextModal, useCtx } from "../context.ts";
import { autofocus } from "@solid-primitives/autofocus";
import type { Media } from "sdk";
import { getHeight, getUrl, getWidth, Resize } from "../media/util.tsx";
import { useApi } from "../api.tsx";
import { createResource } from "solid-js";
import { MessageView } from "../Message.tsx";
import { diffChars } from "diff";
import { ModalResetPassword } from "../user_settings/mod.tsx";

export const Modal = (props: ParentProps) => {
	const ctx = useCtx()!;
	return (
		<div class="modal">
			<div class="bg" onClick={() => ctx.dispatch({ do: "modal.close" })}></div>
			<div class="content">
				<div class="base"></div>
				<div class="inner" role="dialog" aria-modal>
					{props.children}
				</div>
			</div>
		</div>
	);
};

export function getModal(modal: ContextModal) {
	switch (modal.type) {
		case "alert": {
			return <ModalAlert text={modal.text} />;
		}
		case "confirm": {
			return <ModalConfirm text={modal.text} cont={modal.cont} />;
		}
		case "prompt": {
			return <ModalPrompt text={modal.text} cont={modal.cont} />;
		}
		case "media": {
			return <ModalMedia media={modal.media} />;
		}
		case "message_edits": {
			return (
				<ModalMessageEdits
					thread_id={modal.thread_id}
					message_id={modal.message_id}
				/>
			);
		}
		case "reset_password": {
			return <ModalResetPassword />;
		}
	}
}

const ModalAlert = (props: { text: string }) => {
	const ctx = useCtx()!;
	return (
		<Modal>
			<p>{props.text}</p>
			<div class="bottom">
				<button onClick={() => ctx.dispatch({ do: "modal.close" })}>
					okay!
				</button>
			</div>
		</Modal>
	);
};

const ModalConfirm = (
	props: { text: string; cont: (bool: boolean) => void },
) => {
	const ctx = useCtx()!;
	return (
		<Modal>
			<p>{props.text}</p>
			<div class="bottom">
				<button
					onClick={() => {
						props.cont(true);
						ctx.dispatch({ do: "modal.close" });
					}}
				>
					okay!
				</button>
				<button
					onClick={() => {
						props.cont(false);
						ctx.dispatch({ do: "modal.close" });
					}}
				>
					nevermind...
				</button>
			</div>
		</Modal>
	);
};

const ModalPrompt = (
	props: { text: string; cont: (s: string | null) => void },
) => {
	const ctx = useCtx()!;
	return (
		<Modal>
			<p>{props.text}</p>
			<div style="height: 8px"></div>
			<form
				onSubmit={(e) => {
					e.preventDefault();
					const form = e.target as HTMLFormElement;
					const input = form.elements.namedItem(
						"text",
					) as HTMLInputElement;
					props.cont(input.value);
					ctx.dispatch({ do: "modal.close" });
				}}
			>
				<input type="text" name="text" use:autofocus autofocus />
				<div class="bottom">
					<input type="submit" value="done!"></input>{" "}
					<button
						onClick={() => {
							props.cont(null);
							ctx.dispatch({ do: "modal.close" });
						}}
					>
						nevermind...
					</button>
				</div>
			</form>
		</Modal>
	);
};

// currently only supports images!
// though, it doesn't make much sense for video/audio/other media?
const ModalMedia = (props: { media: Media }) => {
	const ctx = useCtx();

	const [loaded, setLoaded] = createSignal(false);
	const height = () => getHeight(props.media);
	const width = () => getWidth(props.media);

	createEffect(() => console.log("loaded", loaded()));
	return (
		<div class="modal modal-media">
			<div class="bg" onClick={() => ctx.dispatch({ do: "modal.close" })}></div>
			<div class="content">
				<div class="base"></div>
				<div class="inner" role="dialog" aria-modal>
					<Resize height={height()} width={width()}>
						<div class="image full">
							<div class="media-loader" classList={{ loaded: loaded() }}>
								loading
							</div>
							<img
								src={getUrl(props.media)}
								alt={props.media.alt ?? undefined}
								height={height()!}
								width={width()!}
								onLoad={[setLoaded, true]}
								onEmptied={[setLoaded, false]}
							/>
						</div>
					</Resize>
					<a href={props.media.source.url}>Go to url</a>
				</div>
			</div>
		</div>
	);
};

const ModalMessageEdits = (
	props: { thread_id: string; message_id: string },
) => {
	// FIXME: pagination
	const api = useApi();
	const [edits] = createResource(
		{ thread_id: props.thread_id, message_id: props.message_id },
		async (path) => {
			const { data } = await api.client.http.GET(
				"/api/v1/thread/{thread_id}/message/{message_id}/version",
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
			<h3>edit history</h3>
			<br />
			<ul>
				<For each={edits()?.items ?? []} fallback={"loading"}>
					{(i, x) => {
						const prev = edits()?.items[x() - 1];
						if (prev) {
							const pages = diffChars(i.content ?? "", prev.content ?? "");
							const content = pages.map((i) => {
								if (i.added) return `<ins>${i.value}</ins>`;
								if (i.removed) return `<del>${i.value}</del>`;
								return i.value;
							}).join("");
							return (
								<li>
									<MessageView message={{ ...i, content }} />
								</li>
							);
						} else {
							return (
								<li>
									<MessageView message={i} />
								</li>
							);
						}
					}}
				</For>
			</ul>
		</Modal>
	);
};
