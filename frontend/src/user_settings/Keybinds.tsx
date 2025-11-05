export function Keybinds() {
	// TODO: list default keybinds
	// TODO: allow rebinding keybinds
	return (
		<div class="user-settings-keybinds">
			<h2>keybinds</h2>
			<h3>global</h3>
			<ul>
				<li>
					<span>Show quick switcher</span>
					<kbd>Ctrl+K</kbd>
				</li>
			</ul>
			<h3>chat</h3>
			<ul>
				<li>
					<span>Search messages</span>
					<kbd>Ctrl+F</kbd>
				</li>
				<li>
					<span>foo</span>
					<kbd>Ctrl+1</kbd>
				</li>
				<li>
					<span>bar</span>
					<kbd>Ctrl+2</kbd>
				</li>
				<li>
					<span>baz</span>
					<kbd>Ctrl+3</kbd>
				</li>
			</ul>
		</div>
	);
}
