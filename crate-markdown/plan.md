# Lamprey Markdown Parser - Development Plan

## High Priority

### Improve Incremental Editing
- [ ] Reuse inline formatting nodes within paragraphs
- [ ] Track dirty regions at character level
- [ ] Benchmark current vs improved performance
- [ ] Add tests verifying tree reuse (count reused nodes)
- [ ] Consider using `rowan::api::SyntaxNode` methods for better reuse

## Medium Priority

### Missing Markdown Features

#### Horizontal Rules
- [ ] Add `HorizontalRule` syntax kind
- [ ] Parse `---`, `***`, `___` (3+ repeated chars)
- [ ] Add tests for each variant
- [ ] Handle mixed variants like `- - -`

#### Task Lists
- [ ] Add `TaskList` / `TaskListItem` syntax kinds
- [ ] Add `TaskMarker` for `[ ]` and `[x]`
- [ ] Parse `- [ ] item` and `- [x] done`
- [ ] Add tests for checked/unchecked items

#### Code Block Language Hints
- [ ] Store language info in `CodeBlock` node
- [ ] Parse ` ```rust ` fence info
- [ ] Add `CodeBlockInfo` syntax kind
- [ ] Add tests for various languages

#### Tables
- [ ] Add `Table`, `TableRow`, `TableCell` syntax kinds
- [ ] Parse header row with `|` separators
- [ ] Parse delimiter row (`|---|---|`)
- [ ] Parse body rows
- [ ] Handle alignment markers (`:---`, `---:`, `:---:`)
- [ ] Add comprehensive table tests

#### Nested Lists
- [ ] Track indentation levels
- [ ] Allow lists inside list items
- [ ] Handle mixed bullet/numbered nesting
- [ ] Add tests for various nesting levels

### HTML Renderer
- [ ] Uncomment/create `src/render/html.rs`
- [ ] Implement `HtmlReader` struct
- [ ] Map syntax kinds to HTML tags:
  - `Strong` → `<strong>`
  - `Emphasis` → `<em>`
  - `Strikethrough` → `<del>`
  - `InlineCode` → `<code>`
  - `Link` → `<a href="...">`
  - `Header` → `<h1>` through `<h6>`
  - `List` → `<ul>` / `<ol>`
  - `BlockQuote` → `<blockquote>`
  - `CodeBlock` → `<pre><code>`
  - `Paragraph` → `<p>`
- [ ] Handle escaping for HTML entities
- [ ] Add tests for each element type

## Lower Priority

### Additional Tests

#### Edge Cases
- [ ] Very long documents (10k+ chars)
- [ ] Deeply nested formatting (bold inside italic inside bold...)
- [ ] Unicode edge cases (emoji, CJK, RTL text)
- [ ] Mixed line endings (`\r\n`, `\n`, `\r`)
- [ ] Trailing whitespace handling
- [ ] Leading/trailing blank lines

#### Performance Tests
- [ ] Benchmark initial parse speed
- [ ] Benchmark incremental edit speed
- [ ] Memory usage tests
- [ ] Compare with other markdown parsers (pulldown-cmark, comrak)

#### Malformed Input
- [ ] Extremely long lines
- [ ] Many consecutive delimiters (`********`)
- [ ] Interleaved delimiters (`**__**__`)
- [ ] Null bytes and control characters

### API Improvements

#### True Reader Composition
- [ ] Make `PlainTextReader` wrap another reader
- [ ] Make `StripEmojiReader` wrap another reader
- [ ] Chain: `IdentityReader.strip_emoji().plain()`
- [ ] Each reader processes output of previous
- [ ] Update trait methods to support composition
- [ ] Add tests for chained readers

#### Better Error Reporting
- [ ] Add `ParseWarning` type for recoverable issues
- [ ] Track unclosed delimiters
- [ ] Track mismatched delimiters
- [ ] Return warnings alongside `Parsed`

#### Source Mapping
- [ ] Add `Span` to each syntax node
- [ ] Map syntax nodes back to source positions
- [ ] Support "go to definition" style lookups
- [ ] Enable syntax highlighting integration

### Documentation

#### README
- [ ] Write crate-markdown/README.md
- [ ] Include feature list
- [ ] Add quick start example

#### Rustdoc
- [ ] Add examples to all public functions
- [ ] Document all public types
- [ ] Add module-level documentation
- [ ] Ensure `cargo doc` produces useful output

#### WASM Support
- [ ] Ensure `wasm` feature works
- [ ] Add wasm-pack build
- [ ] Create JavaScript bindings

## Completed

- [x] Basic inline formatting (bold, italic, strikethrough, code)
- [x] Links (named, autolinks, angle bracket)
- [x] Mentions (`@uuid`, `<@uuid>`)
- [x] Custom emoji (`:name:uuid:`)
- [x] Block elements (headers, lists, blockquotes, code blocks)
- [x] Escape sequences (`\*`, `\[`, `\\`)
- [x] Incremental editing (basic tree reuse)
- [x] Plain text renderer
- [x] Strip emoji renderer
- [x] Identity renderer
- [x] `MarkdownReader` trait
- [x] Comprehensive test suite (81 tests)
- [x] Doc tests (7 passing)
- [x] Public API documentation
- [x] Architecture documentation

## Notes

images are intentionally not planned

### Known Limitations

1. **Inline reuse**: Incremental editing only reuses block-level nodes, not inline formatting within paragraphs.

2. **Reader composition**: The `MarkdownReader` trait methods create wrapper types but don't truly compose - each reader operates on the AST directly rather than chaining output.

3. **Link/URL validation**: URLs are not validated, just tokenized. Invalid URLs will still parse.

4. **No GFM extensions**: GitHub Flavored Markdown tables, task lists, etc. are not yet implemented.

### Design Principles

1. **Error tolerance**: Parser never fails on malformed input. Always produces a valid tree.

2. **Incremental first**: Architecture supports incremental editing, even if not fully utilized yet.

3. **Zero-copy where possible**: Uses `Arc<str>` for source, rowan's immutable trees.

4. **Composable renderers**: Trait-based renderer system allows different output formats.

5. **Wasm compatible**: Uses `u32` for spans, avoids platform-specific code.
