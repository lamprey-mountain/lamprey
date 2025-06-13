import { createEffect, createSignal, type ParentProps, Show } from "solid-js";
import { type Modal as ContextModal, useCtx } from "../context.ts";
import { autofocus } from "@solid-primitives/autofocus";
import type { Media } from "sdk";
import { getHeight, getUrl, getWidth, Resize } from "../media/util.tsx";

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
	}
}

const ModalAlert = (props: { text: string }) => {
	const ctx = useCtx()!;
	return (
		<Modal>
			<p>{props.text}</p>
			<div style="height: 8px"></div>
			<button onClick={() => ctx.dispatch({ do: "modal.close" })}>
				okay!
			</button>
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
			<div style="height: 8px"></div>
			<button
				onClick={() => {
					props.cont(true);
					ctx.dispatch({ do: "modal.close" });
				}}
			>
				okay!
			</button>&nbsp;
			<button
				onClick={() => {
					props.cont(false);
					ctx.dispatch({ do: "modal.close" });
				}}
			>
				nevermind...
			</button>
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
				<div style="height: 8px"></div>
				<input type="submit" value="done!"></input>{" "}
				<button
					onClick={() => {
						props.cont(null);
						ctx.dispatch({ do: "modal.close" });
					}}
				>
					nevermind...
				</button>
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
								src={getUrl(props.media.source)}
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
