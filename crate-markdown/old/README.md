# lamprey markdown

WEE WOO WEE WOO WORK IN PROGRESS DO NOT USE WEE WOO WEE WOO

THIS IS HEAVILY LLM GENERATED AND **HASN'T BEEN REVIEWED** YET

this exists for a few different reasons

- performant syntax highlighting in the markdown editor with incremental
  reparsing
- standardized markdown parsing between the client and server
- for the server to be able to correctly parse mentions (ignoring escapes and
  codeblocks), as well as sanitize custom emoji

## WASM Bindings

This crate includes optional WASM bindings for use in JavaScript/TypeScript
environments. The bindings expose two primary use cases:

1. **SolidJS rendering** — one-shot `text → events` (JSON) for client-side rendering
2. **ProseMirror syntax highlighting** — incremental reparsing with token-level info

### Building the WASM module

Prerequisites:
```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

Build the WASM module:
```bash
wasm-pack build crate-markdown --features wasm --target web
```

This outputs the WASM bundle and JS bindings to `crate-markdown/pkg/`.

### Running the test page

After building, you can serve the test page locally:

```bash
# Using Python
python3 -m http.server 8080 -d crate-markdown/test

# Or using Node's http-server
npx http-server crate-markdown/test -p 8080
```

Then open `http://localhost:8080` in your browser.

### API Overview

```javascript
import init, { parse_markdown, render_markdown, render_plaintext, WasmParsed } from './pkg/lamprey_markdown.js';

await init();

// One-shot parse (SolidJS rendering)
const result = parse_markdown("# Hello **world**");
// Returns JSON: { events: [...], tokens: [...], sourceLength: number }

// Render to markdown string (identity)
const md = render_markdown("# Hello **world**");

// Render to plain text
const text = render_plaintext("# Hello **world**");

// Incremental editing (ProseMirror syntax highlighting)
const parsed = new WasmParsed("# Hello world");
const tokens = JSON.parse(parsed.tokens());

// Edit incrementally (reuses unchanged tree portions)
parsed.edit_and_tokens(2, 7, "Greetings");
```

### Using as a Rust dependency

The crate type includes both `cdylib` (for WASM) and `rlib` (for normal Rust
linkage), so you can still use it as a regular Rust dependency:

```toml
[dependencies]
lamprey-markdown = { path = "crate-markdown" }
```
