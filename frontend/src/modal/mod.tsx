import { ParentProps } from "solid-js";
import { Modal as ContextModal, useCtx } from "../context.ts";
import { autofocus } from "@solid-primitives/autofocus";
import { Media } from "sdk";

export const Modal = (props: ParentProps) => {
	const ctx = useCtx()!;
	return (
		<div class="modal">
			<div class="bg" onClick={() => ctx.dispatch({ do: "modal.close" })}></div>
			<div class="content">
				<div class="base"></div>
				<div class="inner" role="dialog">
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

// currently only suports images!
const ModalMedia = (props: { media: Media }) => {
	const ctx = useCtx();
	return (
		<div class="modal modal-media">
			<div class="bg" onClick={() => ctx.dispatch({ do: "modal.close" })}></div>
			<div class="content">
				<div class="base"></div>
				<div class="inner" role="dialog">
					<div
						class="media"
						style={{
							"--height": `${props.media.height}px`,
							"--width": `${props.media.width}px`,
							"--aspect-ratio": `${props.media.width}/${props.media.height}`,
						}}
					>
						<div class="inner">
							<div class="loader">loading</div>
							<img
								src={props.media.url}
								alt={props.media.alt ?? undefined}
								height={props.media.height!}
								width={props.media.width!}
							/>
						</div>
					</div>
				</div>
			</div>
		</div>
	);
};
