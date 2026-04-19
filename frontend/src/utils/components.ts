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

	// 1. Process deletes
	for (const id of delta.delete) {
		result = recursiveDelete(result, id);
	}

	// 2. Process replacements
	for (const r of delta.replace) {
		result = recursiveReplace(result, r.target, r.components);
	}

	// 3. Process appends
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
	targetId: string,
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
	targetId: string,
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
	targetId: string,
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
	if (parent.type === "Container" || parent.type === "Section") {
		for (const comp of components) {
			parent.components.push(createComponentFromCreate(comp));
		}
	} else if (parent.type === "Details") {
		// Append to details (not summary)
		for (const comp of components) {
			parent.details.push(createComponentFromCreate(comp));
		}
	} else if (parent.type === "Text") {
		// Text components concatenate content
		for (const comp of components) {
			if (comp.type === "Text") {
				parent.content += comp.content;
			}
		}
	} else if (parent.type === "Gallery") {
		// Media can be appended to Gallery
		for (const comp of components) {
			if (comp.type === "Media") {
				parent.items.push(...(comp.items as any));
			}
		}
	}
	// Other component types don't support appending
}

/**
 * Create a LampreyComponent from a LampreyComponentCreate
 */
export function createComponentFromCreate(
	create: LampreyComponentCreate,
): LampreyComponent {
	const base: any = {
		id: create.id ?? crypto.randomUUID(),
	};

	switch (create.type) {
		case "Button":
			return {
				...base,
				type: "Button",
				label: create.label,
				style: create.style,
				custom_id: create.custom_id,
			};
		case "LinkButton":
			return {
				...base,
				type: "LinkButton",
				label: create.label,
				url: create.url,
			};
		case "Container":
			return {
				...base,
				type: "Container",
				components: create.components.map(createComponentFromCreate),
				color: create.color ?? null,
			};
		case "Text":
			return {
				...base,
				type: "Text",
				content: create.content,
			};
		case "Details":
			return {
				...base,
				type: "Details",
				open: create.open,
				color: create.color ?? null,
				summary: create.summary.map(createComponentFromCreate),
				details: create.details.map(createComponentFromCreate),
			};
		case "Section":
			return {
				...base,
				type: "Section",
				color: create.color ?? null,
				components: create.components.map(createComponentFromCreate),
			};
		case "Media":
			return {
				...base,
				type: "Media",
				items: create.items as any,
			};
		case "Gallery":
			return {
				...base,
				type: "Gallery",
				items: create.items as any,
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
	targetId: string,
	appends: LampreyComponentCreate[],
): LampreyComponent[] {
	return components.map((comp): LampreyComponent => {
		if (comp.id === targetId) {
			if (comp.type === "Text") {
				const extraText = appends
					.filter((a) => a.type === "Text")
					.map((a) => (a as any).content)
					.join("");
				return {
					...comp,
					content: comp.content + extraText,
				} as LampreyComponent;
			}

			if (comp.type === "Container" || comp.type === "Section") {
				return {
					...comp,
					components: [
						...comp.components,
						...appends.map(createComponentFromCreate),
					],
				} as LampreyComponent;
			}

			if (comp.type === "Details") {
				return {
					...comp,
					details: [...comp.details, ...appends.map(createComponentFromCreate)],
				} as LampreyComponent;
			}

			if (comp.type === "Gallery") {
				return {
					...comp,
					items: [
						...comp.items,
						// TODO: type FlumeDelta from server differently, we have the canonical component
						...appends.flatMap((a) =>
							a.type === "Gallery" || a.type === "Media"
								? (a.items as any)
								: [],
						),
					],
				} as LampreyComponent;
			}

			return comp;
		}

		if (comp.type === "Container" || comp.type === "Section") {
			const newChildren = recursiveAppend(comp.components, targetId, appends);
			if (newChildren !== comp.components) {
				return { ...comp, components: newChildren } as LampreyComponent;
			}
		} else if (comp.type === "Details") {
			const newSummary = recursiveAppend(comp.summary, targetId, appends);
			const newDetails = recursiveAppend(comp.details, targetId, appends);
			if (newSummary !== comp.summary || newDetails !== comp.details) {
				return {
					...comp,
					summary: newSummary,
					details: newDetails,
				} as LampreyComponent;
			}
		}

		return comp;
	});
}
