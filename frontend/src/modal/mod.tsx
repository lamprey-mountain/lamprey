import { ParentProps } from "solid-js";
import { Modal as ContextModal, useCtx } from "../context.ts";
import { autofocus } from "@solid-primitives/autofocus";

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
	const ctx = useCtx()!;
	switch (modal.type) {
		case "alert": {
			return (
				<Modal>
					<p>{modal.text}</p>
					<div style="height: 8px"></div>
					<button onClick={() => ctx.dispatch({ do: "modal.close" })}>
						okay!
					</button>
				</Modal>
			);
		}
		case "confirm": {
			return (
				<Modal>
					<p>{modal.text}</p>
					<div style="height: 8px"></div>
					<button
						onClick={() => {
							modal.cont(true);
							ctx.dispatch({ do: "modal.close" });
						}}
					>
						okay!
					</button>&nbsp;
					<button
						onClick={() => {
							modal.cont(false);
							ctx.dispatch({ do: "modal.close" });
						}}
					>
						nevermind...
					</button>
				</Modal>
			);
		}
		case "prompt": {
			return (
				<Modal>
					<p>{modal.text}</p>
					<div style="height: 8px"></div>
					<form
						onSubmit={(e) => {
							e.preventDefault();
							const form = e.target as HTMLFormElement;
							const input = form.elements.namedItem(
								"text",
							) as HTMLInputElement;
							modal.cont(input.value);
							ctx.dispatch({ do: "modal.close" });
						}}
					>
						<input type="text" name="text" use:autofocus autofocus />
						<div style="height: 8px"></div>
						<input type="submit" value="done!"></input>{" "}
						<button
							onClick={() => {
								modal.cont(null);
								ctx.dispatch({ do: "modal.close" });
							}}
						>
							nevermind...
						</button>
					</form>
				</Modal>
			);
		}
	}
}
