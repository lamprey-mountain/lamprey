import type {
	FlumeDelta,
	LampreyComponent,
	LampreyComponentCreate,
} from "ts-sdk";

/**
 * Apply a FlumeDelta to a list of components.
 * This mirrors the backend logic in crate-common/src/v1/types/components/validate.rs
 */
export function applyDelta(
	components: LampreyComponent[],
	delta: FlumeDelta,
): LampreyComponent[] {
	let result = [...components];

	// 1. Process init
	if (delta.init) {
		result = delta.init.map((c) =>
			typeof c === "string"
				? { id: 0, type: "Text", content: c }
				: createComponentFromCreate(c),
		);
	}

	// 2. Process deletes
	for (const id of delta.delete) {
		result = recursiveDelete(result, id);
	}

	// 3. Process replacements
	for (const r of delta.replace) {
		result = recursiveReplace(result, r.target, r.components);
	}

	// 4. Process appends
	for (const a of delta.append) {
		result = recursiveAppend(result, a.target, a.components);
	}

	return result;
}

/**
 * Recursively delete a component by ID
 */
export function recursiveDelete(
	components: LampreyComponent[],
	targetId: number,
): LampreyComponent[] {
	const result: LampreyComponent[] = [];

	for (const comp of components) {
		if (comp.id === targetId) {
			// Skip this component (delete it)
			continue;
		}

		// Recursively process children
		const newComp = { ...comp };

		if (newComp.type === "Container" || newComp.type === "Section") {
			newComp.components = recursiveDelete(newComp.components, targetId);
		} else if (newComp.type === "Details") {
			newComp.summary = recursiveDelete(newComp.summary, targetId);
			newComp.details = recursiveDelete(newComp.details, targetId);
		}

		result.push(newComp);
	}

	return result;
}

/**
 * Recursively replace a component by ID with new components
 */
export function recursiveReplace(
	components: LampreyComponent[],
	targetId: number,
	replacements: LampreyComponentCreate[],
): LampreyComponent[] {
	const result: LampreyComponent[] = [];

	for (const comp of components) {
		if (comp.id === targetId) {
			// Replace this component with the new ones
			for (const repl of replacements) {
				result.push(createComponentFromCreate(repl));
			}
		} else {
			// Recursively process children
			const newComp = { ...comp };

			if (newComp.type === "Container" || newComp.type === "Section") {
				newComp.components = recursiveReplace(
					newComp.components,
					targetId,
					replacements,
				);
			} else if (newComp.type === "Details") {
				newComp.summary = recursiveReplace(
					newComp.summary,
					targetId,
					replacements,
				);
				newComp.details = recursiveReplace(
					newComp.details,
					targetId,
					replacements,
				);
			}

			result.push(newComp);
		}
	}

	return result;
}

/**
 * Find a component by ID recursively
 */
export function findComponentById(
	components: LampreyComponent[],
	targetId: number,
): LampreyComponent | null {
	for (const comp of components) {
		if (comp.id === targetId) {
			return comp;
		}

		// Search in children
		if (comp.type === "Container" || comp.type === "Section") {
			const found = findComponentById(comp.components, targetId);
			if (found) return found;
		} else if (comp.type === "Details") {
			const found = findComponentById(comp.summary, targetId);
			if (found) return found;
			const found2 = findComponentById(comp.details, targetId);
			if (found2) return found2;
		}
	}

	return null;
}

/**
 * Append components to a parent component
 */
export function appendComponents(
	parent: LampreyComponent,
	components: LampreyComponentCreate[],
): void {
	for (const comp of components) {
		const parsed: LampreyComponent =
			typeof comp === "string"
				? { id: 0, type: "Text", content: comp }
				: createComponentFromCreate(comp);
		if (parent.type === "Container" || parent.type === "Section") {
			parent.components.push(parsed);
		} else if (parent.type === "Details") {
			parent.details.push(parsed);
		} else if (parent.type === "Text") {
			// Text components concatenate content
			if (parsed.type === "Text") {
				parent.content += parsed.content;
			}
		} else if (parent.type === "Gallery") {
			// Media can be appended to Gallery
			if (parsed.type === "Media") {
				parent.items.push(...parsed.items);
			}
		}
		// Other component types don't support appending
	}
}

/**
 * Create a LampreyComponent from a LampreyComponentCreate
 */
export function createComponentFromCreate(
	create: LampreyComponentCreate,
): LampreyComponent {
	const id: number =
		typeof create === "string"
			? 0
			: (create.id ?? Math.floor(Math.random() * 0xffff));

	if (typeof create === "string") {
		return { id, type: "Text", content: create };
	}

	switch (create.type) {
		case "Button":
			return {
				id,
				type: "Button",
				label: create.label,
				style: create.style,
				custom_id: create.custom_id,
			};
		case "LinkButton":
			return {
				id,
				type: "LinkButton",
				label: create.label,
				url: create.url ?? null,
			};
		case "Container":
			return {
				id,
				type: "Container",
				components: create.components.map(createComponentFromCreate),
				color: create.color ?? null,
			};
		case "Text":
			return {
				id,
				type: "Text",
				content: create.content,
			};
		case "Details":
			return {
				id,
				type: "Details",
				open: create.open,
				color: create.color ?? null,
				summary: create.summary.map(createComponentFromCreate),
				details: create.details.map(createComponentFromCreate),
			};
		case "Section":
			return {
				id,
				type: "Section",
				color: create.color ?? null,
				components: create.components.map(createComponentFromCreate),
			};
		case "Media":
			return {
				id,
				type: "Media",
				items: create.items.map((item) => ({
					media: { id: item.media_id } as any,
					description: item.description,
					spoiler: item.spoiler,
				})),
			};
		case "Gallery":
			return {
				id,
				type: "Gallery",
				items: create.items.map((item) => ({
					media: { id: item.media_id } as any,
					description: item.description,
					spoiler: item.spoiler,
				})),
			};
		default:
			// Should never happen with proper typing
			throw new Error(`Unknown component type: ${(create as any).type}`);
	}
}

/**
 * Recursively find target and append components immutably
 */
export function recursiveAppend(
	components: LampreyComponent[],
	targetId: number,
	appends: LampreyComponentCreate[],
): LampreyComponent[] {
	return components.map((comp): LampreyComponent => {
		if (comp.id === targetId) {
			if (comp.type === "Text") {
				const extraText = appends
					.map((a) => {
						const parsed =
							typeof a === "string" ? a : a.type === "Text" ? a.content : null;
						return parsed ?? "";
					})
					.join("");
				return {
					...comp,
					content: comp.content + extraText,
				};
			}

			if (comp.type === "Container" || comp.type === "Section") {
				return {
					...comp,
					components: [
						...comp.components,
						...appends.map(createComponentFromCreate),
					],
				};
			}

			if (comp.type === "Details") {
				return {
					...comp,
					details: [...comp.details, ...appends.map(createComponentFromCreate)],
				};
			}

			if (comp.type === "Gallery") {
				return {
					...comp,
					items: [
						...comp.items,
						...appends.flatMap((a) => {
							const parsed = createComponentFromCreate(a);
							if (parsed.type === "Media") {
								return parsed.items;
							}
							return [];
						}),
					],
				};
			}

			return comp;
		}

		if (comp.type === "Container" || comp.type === "Section") {
			const newChildren = recursiveAppend(comp.components, targetId, appends);
			if (newChildren !== comp.components) {
				return { ...comp, components: newChildren };
			}
		} else if (comp.type === "Details") {
			const newSummary = recursiveAppend(comp.summary, targetId, appends);
			const newDetails = recursiveAppend(comp.details, targetId, appends);
			if (newSummary !== comp.summary || newDetails !== comp.details) {
				return {
					...comp,
					summary: newSummary,
					details: newDetails,
				};
			}
		}

		return comp;
	});
}
