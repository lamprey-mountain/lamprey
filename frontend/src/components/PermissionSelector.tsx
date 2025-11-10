import type { Permission } from "sdk";
import { type Component, createMemo, createSignal, For } from "solid-js";
import icCheck1 from "../assets/check-1.png";
import icCheck2 from "../assets/check-2.png";
import icCheck3 from "../assets/check-3.png";
import icCheck4 from "../assets/check-4.png";
import icSlash1 from "../assets/slash-1.png";
import icSlash2 from "../assets/slash-2.png";
import icSlash3 from "../assets/slash-3.png";
import icSlash4 from "../assets/slash-4.png";
import icX1 from "../assets/x-1.png";
import icX2 from "../assets/x-2.png";
import icX3 from "../assets/x-3.png";
import icX4 from "../assets/x-4.png";
import { cyrb53 } from "../colors.ts";
import { LCG } from "../pfp.ts";
import { permissions } from "../permissions.ts";

const icon = (type: "x" | "slash" | "check", seed: string) => {
	const rand = LCG(cyrb53(seed));
	function rnd<T>(arr: T[]): T {
		return arr[Math.floor(rand() * arr.length)];
	}

	switch (type) {
		case "x":
			return rnd([icX1, icX2, icX3, icX4]);
		case "slash":
			return rnd([icSlash1, icSlash2, icSlash3, icSlash4]);
		case "check":
			return rnd([icCheck1, icCheck2, icCheck3, icCheck4]);
	}
};

type PermState = "allow" | "deny" | "inherit";
type PermissionItem = {
	id: Permission;
	name: string;
	description: string;
	group?: string;
};

interface PermissionSelectorProps {
	seed: string;
	permissions: PermissionItem[];
	permStates: Record<Permission, PermState>;
	onPermChange: (perm: Permission, state: PermState) => void;
	showDescriptions?: boolean;
	roomType?: "Default" | "Server";
}

export const PermissionSelector: Component<PermissionSelectorProps> = (
	props,
) => {
	const [search, setSearch] = createSignal("");

	const filteredPermissions = createMemo(() => {
		const searchTerm = search().toLowerCase();
		if (!searchTerm) return props.permissions;
		return props.permissions.filter((p) =>
			p.name.toLowerCase().includes(searchTerm) ||
			p.description.toLowerCase().includes(searchTerm) ||
			p.id.toLowerCase().includes(searchTerm)
		);
	});

	const groupedPermissions = createMemo(() => {
		const filtered = filteredPermissions();
		const groups = new Map<string, PermissionItem[]>();

		const groupOrder = props.roomType === "Default"
			? [
				"room",
				"members",
				"messages",
				"channels",
				"voice",
				"calendar",
				"dangerous",
			]
			: [
				"server",
				"server members",
				"room",
				"members",
				"messages",
				"channels",
				"voice",
				"calendar",
				"dangerous",
			];

		groupOrder.forEach((group) => {
			groups.set(group, []);
		});

		for (const perm of filtered) {
			const group = perm.group;
			if (group && groups.has(group)) {
				groups.get(group)!.push(perm);
			}
		}

		const list = [];
		for (const group of groupOrder) {
			const ps = groups.get(group);
			if (ps?.length === 0) continue;
			list.push({ group, perms: ps });
		}

		return list;
	});

	return (
		<div class="permission-selector">
			<input
				type="search"
				placeholder="Search permissions..."
				value={search()}
				onInput={(e) => setSearch(e.currentTarget.value)}
				class="permission-search-input"
			/>
			<div class="permission-selector-list">
				<For each={groupedPermissions()}>
					{({ group, perms }) => {
						return (
							<div class="permission-group">
								<h3>{group}</h3>
								<ul>
									<For each={perms}>
										{(p) => {
											const state = createMemo(() =>
												props.permStates[p.id] || "inherit"
											);
											const [isExpanded, setIsExpanded] = createSignal(false);

											return (
												<li class="permission-item">
													<div class="permission-info">
														<div class="permission-name">{p.name}</div>
														{props.showDescriptions && (
															<div
																class="permission-description"
																onClick={() => setIsExpanded(!isExpanded())}
															>
																{isExpanded()
																	? p.description
																	: p.description.substring(0, 100) +
																		(p.description.length > 100 ? "..." : "")}
															</div>
														)}
													</div>
													<div class="permission-controls">
														<button
															class="perm-state-button"
															classList={{
																"state-allow": state() === "allow",
															}}
															onClick={() => props.onPermChange(p.id, "allow")}
															title="Allow"
														>
															<img
																class="icon"
																src={icon("check", props.seed + p.id)}
															/>
														</button>
														<button
															class="perm-state-button"
															classList={{
																"state-inherit": state() === "inherit",
															}}
															onClick={() =>
																props.onPermChange(p.id, "inherit")}
															title="Default"
														>
															<img
																class="icon"
																src={icon("slash", props.seed + p.id)}
															/>
														</button>
														<button
															class="perm-state-button"
															classList={{
																"state-deny": state() === "deny",
															}}
															onClick={() => props.onPermChange(p.id, "deny")}
															title="Deny"
														>
															<img
																class="icon"
																src={icon("x", props.seed + p.id)}
															/>
														</button>
													</div>
												</li>
											);
										}}
									</For>
								</ul>
							</div>
						);
					}}
				</For>
			</div>
		</div>
	);
};

export { permissions };
