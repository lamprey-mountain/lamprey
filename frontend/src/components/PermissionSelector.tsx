import type { Permission } from "sdk";
import {
	type Component,
	createMemo,
	createSignal,
	For,
	type JSX,
} from "solid-js";
import { permissions } from "../permissions.ts";

type PermState = "allow" | "deny" | "inherit";
type PermissionItem = {
	id: Permission;
	name: string;
	description: string;
	group?: string;
};

interface PermissionSelectorProps {
	permissions: PermissionItem[];
	permStates: Record<Permission, PermState>;
	onPermChange: (perm: Permission, state: PermState) => void;
	showDescriptions?: boolean;
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

	return (
		<div class="permission-selector">
			<input
				type="search"
				placeholder="Search permissions..."
				value={search()}
				onInput={(e) => setSearch(e.currentTarget.value)}
				class="permission-search-input"
			/>
			<ul class="permission-selector-list">
				<For each={filteredPermissions()}>
					{(p) => {
						const state = () => props.permStates[p.id] || "inherit";
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
										✓
									</button>
									<button
										class="perm-state-button"
										classList={{
											"state-inherit": state() === "inherit",
										}}
										onClick={() => props.onPermChange(p.id, "inherit")}
										title="Default"
									>
										/
									</button>
									<button
										class="perm-state-button"
										classList={{
											"state-deny": state() === "deny",
										}}
										onClick={() => props.onPermChange(p.id, "deny")}
										title="Deny"
									>
										X
									</button>
								</div>
							</li>
						);
					}}
				</For>
			</ul>
		</div>
	);
};

export { permissions };
