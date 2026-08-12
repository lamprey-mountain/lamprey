import type { EditorState } from "prosemirror-state";
import { createUpload } from "sdk";
import { Show } from "solid-js";
import { useApi } from "@/api";
import { CheckboxOption } from "@/atoms/CheckboxOption";
import { Checkbox } from "@/atoms/icons";
import { Avatar } from "@/components/shared/User";
import { useAutocomplete } from "@/contexts/autocomplete";
import { useFormattingToolbar } from "@/contexts/formatting-toolbar";
import { useMenu } from "@/contexts/mod";
import { Copyable } from "@/utils/general";
import { createEditor } from "../editor/Editor";
import { serializeToMarkdown } from "../editor/serializer";
import { BridgeSettings } from "./Bridge";
import { type ApplicationDraft, useApplications } from "./context";

export const Overview = (props: { draft: ApplicationDraft }) => {
	const api = useApi();
	const menu = useMenu();
	const apps = useApplications();

	const isCreate = () => props.draft.state === "create";

	const id = () => {
		if (props.draft.state === "create") return props.draft.nonce;
		return props.draft.data.id;
	};

	const currentData = () => {
		const d = props.draft;
		if (d.state === "create") return d.create;
		if (d.state === "update") return { ...d.data, ...d.update };
		return d.data;
	};

	const updateApp = (key: string, value: any) => {
		apps.updateDraft(id(), { [key]: value });
	};

	const setAvatarFile = async (f: File) => {
		await createUpload({
			client: api.client,
			file: f,
			onComplete(media) {
				updateApp("avatar", media.id);
			},
			onFail(_error) {},
			onPause() {},
			onResume() {},
			onProgress(_progress) {},
		});
	};

	const removeAvatar = async () => {
		updateApp("avatar", null);
	};

	const deleteApp = async () => {
		const d = props.draft;
		if (d.state === "create") {
			apps.update((prev) => prev.filter((a) => a !== d));
		} else {
			apps.update((a) => a.data.id === d.data.id, "state", "delete");
		}
	};

	let avatarInputEl!: HTMLInputElement;

	const openAvatarPicker = () => {
		avatarInputEl?.click();
	};

	const appWithAvatar = () => {
		const data = currentData();

		return {
			id: id(),
			name: data.name,
			avatar: (data as any).avatar ?? null,
			banner: null,
			description: null,
			bot: false,
			system: false,
			version_id: "",
			flags: 0,
			presence: { status: "Offline" as const, activities: [] },
			preferences: null,
		};
	};

	const toolbar = useFormattingToolbar();
	const autocomplete = useAutocomplete();

	const descriptionEditor = createEditor({
		channelId: () => id(),
		autocomplete,
		toolbar,
		initialContent: () => currentData().description ?? "",
	});

	const handleDescriptionChange = (state: EditorState) => {
		const s = serializeToMarkdown(state.doc);
		updateApp("description", s || null);
	};

	return (
		<>
			<h3>overview</h3>
			<div class="user-settings-profile profile">
				<div class="name">
					<h3 class="label dim">name</h3>
					<input
						type="text"
						value={currentData().name}
						onInput={(e) => {
							updateApp("name", e.currentTarget.value);
						}}
					/>
				</div>
				<div class="description">
					<h3 class="label dim">description</h3>
					<descriptionEditor.View
						placeholder="application description..."
						submitOnEnter={false}
						onChange={handleDescriptionChange}
						channelId={id()}
						autofocus={false}
					/>
				</div>
				<div class="avatar-uploader" onClick={openAvatarPicker}>
					<div class="avatar-inner">
						<Avatar user={appWithAvatar()} />
						<div class="overlay">upload avatar</div>
					</div>
					<Show when={currentData().avatar}>
						<button
							type="button"
							class="button remove"
							onClick={(e) => {
								e.stopPropagation();
								removeAvatar();
							}}
						>
							remove
						</button>
					</Show>
					<input
						style="display:none"
						ref={avatarInputEl}
						type="file"
						onInput={(e) => {
							const f = e.target.files?.[0];
							if (f) setAvatarFile(f);
						}}
					/>
				</div>
			</div>
			<Show when={!isCreate()}>
				<div>
					id <Copyable>{props.draft.data.id!}</Copyable>
				</div>
			</Show>
			<div style="height: 8px" />
			<CheckboxOption
				id={`app-${id()}-bridge`}
				checked={!!currentData().bridge}
				onChange={(checked) => {
					updateApp("bridge", checked ? {} : null);
				}}
				seed={`app-${id()}-bridge`}
			>
				<Checkbox
					checked={!!currentData().bridge}
					seed={`app-${id()}-bridge`}
				/>
				<label for={`app-${id()}-bridge`} style="display: block">
					<div>bridge</div>
					<div class="dim">can create puppets</div>
				</label>
			</CheckboxOption>
			<Show when={currentData().bridge}>
				<BridgeSettings draft={props.draft} />
			</Show>
			<CheckboxOption
				id={`app-${id()}-public`}
				checked={!!currentData().public}
				onChange={(checked) => {
					updateApp("public", checked);
				}}
				seed={`app-${id()}-public`}
			>
				<Checkbox
					checked={!!currentData().public}
					seed={`app-${id()}-public`}
				/>
				<label for={`app-${id()}-public`} style="display: block">
					<div>public</div>
					<div class="dim">anyone can add and use this bot</div>
				</label>
			</CheckboxOption>
			<br />
			<Show when={!isCreate()}>
				<button
					type="button"
					class="button"
					onClick={(e) => {
						e.stopImmediatePropagation();
						menu.setMenu({
							type: "invite_application",
							app: props.draft.data,
							x: e.clientX,
							y: e.clientY,
						});
					}}
				>
					Add to Room
				</button>
			</Show>
			<button type="button" class="button danger" onClick={deleteApp}>
				Delete Application
			</button>
		</>
	);
};
