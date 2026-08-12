import { CodeBlock } from "@/atoms/Markdown";

// @ts-expect-error
const packageJson = __VITE_PACKAGE_JSON__;

// @ts-expect-error
const gitCommit = __VITE_GIT_COMMIT__;

// @ts-expect-error
const gitDirty = __VITE_GIT_DIRTY__;

export const BuildInfo = () => {
	return (
		<div>
			<h2>build info</h2>
			<p>
				commit {gitCommit} {gitDirty && "(dirty)"}
			</p>
			<CodeBlock
				text={JSON.stringify(packageJson, null, 4)}
				lang="json"
				name="package.json"
			/>
			<h3 class="dim">colors</h3>
			<ul class="debug-colors">
				<li>
					<div class="colored red"></div> red
				</li>
				<li>
					<div class="colored green"></div> green
				</li>
				<li>
					<div class="colored yellow"></div> yellow
				</li>
				<li>
					<div class="colored blue"></div> blue
				</li>
				<li>
					<div class="colored magenta"></div> magenta
				</li>
				<li>
					<div class="colored cyan"></div> cyan
				</li>
				<li>
					<div class="colored orange"></div> orange
				</li>
				<li>
					<div class="colored teal"></div> teal
				</li>
			</ul>
		</div>
	);
};
