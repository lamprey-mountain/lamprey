import type { EditorState } from "prosemirror-state";
import { createUpload, type User } from "sdk";
import {
	createMemo,
	createSignal,
	onMount,
	Show,
	type VoidProps,
} from "solid-js";
import { useApi } from "@/api";
import { useCtx } from "@/app/context";
import { Savebar } from "@/atoms/Savebar";
import { Avatar } from "@/components/shared/User";
import { useAutocomplete } from "@/contexts/autocomplete";
import { useFormattingToolbar } from "@/contexts/formatting-toolbar";
import { useModals } from "@/contexts/modal";
import { Copyable } from "@/utils/general";
import { createEditor } from "../editor/Editor";
import { serializeToMarkdown } from "../editor/serializer";

// TODO(#753): allow uploading banner

export function Profile(props: VoidProps<{ user: User }>) {
	const api2 = useApi();
	const ctx = useCtx();
	const [, _modalCtl] = useModals();

	const [editingName, setEditingName] = createSignal(props.user.name);
	const [editingDescription, setEditingDescription] = createSignal(
		props.user.description,
	);
	const [editingAvatar, setEditingAvatar] = createSignal(props.user.avatar);
	const [editingBanner, setEditingBanner] = createSignal(props.user.banner);

	const toolbar = useFormattingToolbar();
	const autocomplete = useAutocomplete();

	const descriptionEditor = createEditor({
		channelId: () => props.user.id,
		autocomplete,
		toolbar,
		initialContent: () => editingDescription() ?? "",
	});

	const [desc, setDesc] = createSignal(props.user.description ?? "");

	const handleDescriptionChange = (state: EditorState) => {
		setDesc(serializeToMarkdown(state.doc));
	};

	const isDirty = () =>
		editingName() !== props.user.name ||
		desc() !== (props.user.description ?? "") ||
		editingAvatar() !== props.user.avatar ||
		editingBanner() !== props.user.banner;

	const save = async () => {
		await ctx.client.http.PATCH("/api/v1/user/{user_id}", {
			params: { path: { user_id: "@self" } },
			body: {
				name: editingName(),
				description: desc(),
				avatar: editingAvatar(),
				banner: editingBanner(),
			},
		});
	};

	const reset = () => {
		setEditingName(props.user.name);
		setEditingDescription(props.user.description);
		setEditingAvatar(props.user.avatar);
		setEditingBanner(props.user.banner);
		if (descriptionEditor.view) {
			const state = descriptionEditor.createState();
			descriptionEditor.setState(state);
		}
	};

	const setAvatarFile = async (f: File) => {
		await createUpload({
			client: api2.client,
			file: f,
			onComplete(media) {
				setEditingAvatar(media.id);
			},
			onFail(_error) {},
			onPause() {},
			onResume() {},
			onProgress(_progress) {},
		});
	};

	const removeAvatar = async () => {
		setEditingAvatar(null);
	};

	const setBannerFile = async (f: File) => {
		await createUpload({
			client: api2.client,
			file: f,
			onComplete(media) {
				setEditingBanner(media.id);
			},
			onFail(_error) {},
			onPause() {},
			onResume() {},
			onProgress(_progress) {},
		});
	};

	const removeBanner = async () => {
		setEditingBanner(null);
	};

	let avatarInputEl!: HTMLInputElement;

	const openAvatarPicker = () => {
		avatarInputEl?.click();
	};

	let bannerInputEl!: HTMLInputElement;

	const openBannerPicker = () => {
		bannerInputEl?.click();
	};

	const userWithAvatar = () => ({
		id: props.user.id,
		name: props.user.name,
		avatar: editingAvatar(),
		banner: null,
		description: null,
		bot: false,
		system: false,
		version_id: "",
		flags: 0,
		presence: { status: "Offline" as const, activities: [] },
		preferences: null,
	});

	return (
		<div class="user-settings-info">
			<h2>profile</h2>
			<div class="profile">
				<input
					class="name"
					type="text"
					value={editingName()}
					onInput={(e) => setEditingName(e.target.value)}
				/>
				<div class="description">
					<descriptionEditor.View
						placeholder="user description (bio)..."
						submitOnEnter={false}
						onChange={handleDescriptionChange}
						channelId={props.user.id}
						autofocus={false}
					/>
				</div>
				<div class="avatar-uploader" onClick={openAvatarPicker}>
					<div class="avatar-inner">
						<Avatar user={userWithAvatar()} />
						<div class="overlay">upload avatar</div>
					</div>
					<Show when={editingAvatar()}>
						<button
							type="button"
							class="remove"
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
			<div>
				user id: <Copyable>{props.user.id}</Copyable>
			</div>
			<button class="button" onClick={openBannerPicker}>
				upload banner
			</button>
			<button class="button" onClick={removeBanner}>
				remove banner
			</button>
			<input
				style="display:none"
				ref={bannerInputEl}
				type="file"
				onInput={(e) => {
					const f = e.target.files?.[0];
					if (f) setBannerFile(f);
				}}
			/>
			<Savebar show={isDirty()} onCancel={reset} onSave={save} />
		</div>
	);
}
