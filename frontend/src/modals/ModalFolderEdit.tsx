import { createSignal } from "solid-js";
import { useCtx } from "@/app/context";
import { ColorPickerButton } from "@/atoms/ColorPicker";
import { useModals } from "@/contexts/modal";
import type { RoomNavItem } from "@/types/room-nav";
import { Modal } from "./mod";

export const ModalFolderEdit = (props: { folder_id: string }) => {
	const ctx = useCtx();
	const [, modalCtl] = useModals();

	const getFolder = () => {
		const config = (ctx.preferences().frontend.roomNav ?? []) as RoomNavItem[];
		const item = config.find(
			(item) => item.type === "folder" && item.id === props.folder_id,
		);
		return item?.type === "folder" ? item : undefined;
	};

	const [name, setName] = createSignal(getFolder()?.name ?? "");
	const [color, setColor] = createSignal<string | undefined>(
		getFolder()?.color,
	);

	const save = (e: SubmitEvent) => {
		e.preventDefault();
		const folderName = name().trim();
		if (!folderName) return;

		const prefs = ctx.preferences();
		const roomNav = ((prefs.frontend.roomNav ?? []) as RoomNavItem[]).map(
			(item) =>
				item.type === "folder" && item.id === props.folder_id
					? { ...item, name: folderName, color: color() }
					: item,
		);

		ctx.setPreferences({
			...prefs,
			frontend: {
				...prefs.frontend,
				roomNav,
			},
		});
		modalCtl.close();
	};

	return (
		<Modal
			onKeyDown={(e) => {
				if (e.key === "Escape") modalCtl.close();
			}}
		>
			<h3>edit folder</h3>
			<form style="margin-top:8px;" onSubmit={save}>
				<h3 class="dim">name</h3>
				<input
					autofocus
					ref={(el) => queueMicrotask(() => el.focus())}
					type="text"
					value={name()}
					onInput={(e) => setName(e.currentTarget.value)}
					required
				/>
				<h3 class="dim" style="margin-top:8px;">
					color
				</h3>
				<ColorPickerButton value={color()} onInput={setColor} />
				<div class="bottom">
					<button type="button" class="button" onClick={modalCtl.close}>
						cancel
					</button>
					<button type="submit" class="button primary">
						save
					</button>
				</div>
			</form>
		</Modal>
	);
};
