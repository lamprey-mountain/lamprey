# Lamprey Markdown Parser Architecture

A markdown parser library built on top of `rowan` (incremental tree parser) and `logos` (lexer).

## Overview

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Source    │────▶│   Lexer     │────▶│   Tokens    │
│  (markdown) │     │   (logos)   │     │             │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                               │
                                               ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Output    │◀────│   Renderer  │◀────│  Syntax     │
│  (text/HTML)│     │  (readers)  │     │   Tree      │
└─────────────┘     └─────────────┘     │  (rowan)    │
                                        └─────────────┘
```

## Core Components

### 1. Lexer (`TokenKind`)

Located in `src/parser.rs`, uses `logos` for fast tokenization.

**Token Categories:**
- **Whitespace**: spaces, tabs, newlines
- **Delimiters**: `**`, `*`, `~~`, `` ` ``, `[`, `]`, `(`, `)`, `<`, `>`
- **Block markers**: `#` (headers), `-` (lists), `>` (blockquotes)
- **Special patterns**: URLs, UUIDs (for mentions/emoji)
- **Text**: any other content

### 2. Syntax Tree (`SyntaxKind`)

Rowan-based green tree with these node types:

**Block Elements:**
- `Paragraph` - regular text blocks
- `Header` - `#` through `######` headers
- `List` / `ListItem` - bullet and numbered lists
- `BlockQuote` - `>` quoted text
- `CodeBlock` - fenced code blocks

**Inline Elements:**
- `Text` - plain text content
- `Strong` / `StrongDelimiter` - `**bold**`
- `Emphasis` / `EmphasisDelimiter` - `*italic*`
- `Strikethrough` / `StrikethroughDelimiter` - `~~deleted~~`
- `InlineCode` / `InlineCodeFence` / `InlineCodeContent` - `` `code` ``
- `Link` / `LinkText` / `LinkDestination` - `[text](url)`
- `Autolink` - bare URLs
- `AngleBracketLink` - `<https://...>`
- `Mention` / `MentionMarker` - `@uuid`, `<@uuid>`
- `Emoji` / `EmojiName` / `EmojiMarker` - `:name:uuid:`
- `Escape` / `EscapedChar` - `\*`, `\[`, etc.

### 3. Parser

Two-phase parsing in `src/parser.rs`:

**Phase 1: Block Structure**
- `parse()` - main entry point
- `parse_header()` - `# Header`
- `parse_list()` - `- item` or `1. item`
- `parse_blockquote()` - `> quote`
- `parse_code_block()` - ` ```code``` `
- `parse_paragraph()` - default for inline content

**Phase 2: Inline Content**
- `parse_inline()` - recursive descent for inline formatting
- Handles nested formatting (bold inside italic, etc.)
- Manages delimiter matching with `find_closing_*` helpers

### 4. Incremental Editing

Located in `parse_incremental()` function.

**Algorithm:**
1. Walk old tree, identify nodes before/after edit region
2. Clone unaffected nodes directly into new tree
3. Reparse only the affected region
4. Stitch together reused and new nodes

**Current Limitations:**
- Only reuses top-level block nodes
- Doesn't reuse inline formatting within paragraphs
- Could be improved with finer-grained dirty tracking

### 5. Renderers (`src/render/`)

Readers implement the `MarkdownReader` trait:

```rust
pub trait MarkdownReader: Sized {
    fn read(&self, ast: &Ast) -> String;
    fn plain(self) -> PlainTextReader<Self>;
    fn strip_emoji(self, allowed: Vec<EmojiId>) -> StripEmojiReader<Self>;
}
```

**Implementations:**
- `IdentityReader` - returns original source unchanged
- `PlainTextReader` - strips all formatting, returns plain text
- `StripEmojiReader` - filters custom emoji by allowed list

**Usage:**
```rust
let parser = Parser::default();
let parsed = parser.parse("**hello** world");
let ast = Ast::new(parsed);

// Direct usage
let text = PlainTextReader::new().read(&ast);

// Via trait
let text = IdentityReader.plain().read(&ast);
```

## Data Flow

```
1. Parser::parse(source: &str) → Parsed
2. Parsed contains:
   - GreenNode (immutable tree)
   - Arc<str> (original source)
3. Ast wraps Parsed for convenient access
4. Readers consume Ast and produce String output
```

## Key Design Decisions

### Why Rowan?
- Incremental parsing support
- Immutable green trees (efficient editing)
- Text-size based addressing (no line/column tracking)
- Used by rust-analyzer, proven at scale

### Why Logos?
- Fast lexer generation
- Handles token priorities automatically
- Spanned tokens for source mapping

### Error Tolerance
- Parser never fails on malformed input
- Unclosed delimiters become plain text
- Mismatched delimiters are ignored
- Always produces a valid tree

### Escape Handling
- Backslash escapes special characters
- `\*` prevents emphasis, outputs `*`
- `\\` outputs literal backslash
- Escape nodes preserve the escaped character in plain text output

## File Structure

```
crate-markdown/
├── src/
│   ├── lib.rs          # Module exports, re-exports
│   ├── ast.rs          # Ast wrapper type
│   ├── parser.rs       # Lexer, parser, incremental edit
│   └── render/
│       ├── mod.rs      # MarkdownReader trait
│       ├── identity.rs # IdentityReader
│       ├── plain.rs    # PlainTextReader
│       └── strip_emoji.rs # StripEmojiReader
├── tests.rs            # Unit tests (81 tests)
├── architecture.md     # This file
└── plan.md            # TODO list
```

## Public API

```rust
// Core types
pub use ast::Ast;
pub use parser::{Parser, ParseOptions, Parsed, Edit, SyntaxKind, TokenKind};
pub use render::{IdentityReader, PlainTextReader, StripEmojiReader, MarkdownReader};
```

## Performance Characteristics

- **Initial parse**: O(n) where n is source length
- **Incremental edit**: O(k) where k is changed region size (in theory; currently O(n) for inline content)
- **Tree reuse**: Proportional to unchanged content
- **Memory**: GreenNode is immutable and shareable via Arc

## Future Extensions

Potential additions marked with `// TODO` in code:
- HTML renderer
- Table parsing
- Task lists
- Nested lists
- Better incremental editing (inline reuse)
- True reader composition (output of one feeds into next)
