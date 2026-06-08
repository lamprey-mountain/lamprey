use crate::grammar::Table;
use crate::parser::config::ParserConfig;
use crate::prelude::*;

use crate::tokenizer::Tokenizer;
use crate::tree::cursor::TreeCursor;
use crate::tree::node::{NodeKind, TextKind};
use crate::tree::{Cache, Tree, TreeBuilder};

pub mod config;

/// a markdown parser
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct Parser {
    // TODO: remove this?
    // /// core glr state machine definition
    // table: Ref<Table>,
    // TODO: Parser config or other static data
}

/// parsed markdown
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct Parsed {
    config: ParserConfig,
    tree: Tree,
}

/// the result of an edit
pub struct EditResult {
    // TODO: maybe have added/removed Decorations?
}

// pub struct Stack {
//     state_id: u32,
//     // TODO: symbol stacks, lookaheads, etc.
// }

// pub struct ParseContext {
//     /// all current glr branches
//     ///
//     /// contains one item if unambiguous
//     stacks: Vec<Stack>,
// }

// TODO: doc comment
pub struct ParseContext<'a> {
    builder: TreeBuilder,
    tokenizer: Tokenizer<'a>,
    cache: Option<Cache<'a>>,
    pos: Len,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Parser {
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new() -> Self {
        Self {}
    }

    /// parse some markdown
    pub fn parse(&self, markdown: &str) -> Parsed {
        self.parse_with_config(markdown, ParserConfig::default())
    }

    /// parse some markdown with config
    pub fn parse_with_config(&self, markdown: &str, config: ParserConfig) -> Parsed {
        let mut ctx = ParseContext::new(markdown, None);
        let tree = ctx.parse_document();
        Parsed { config, tree }
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Parsed {
    /// get the source string
    pub fn source(&self) -> String {
        self.tree.source().to_string()
    }
}

impl Parsed {
    /// get the syntax tree
    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    /// get a cursor for the syntax tree
    // TODO: wasm compat
    pub fn cursor<'a>(&'a self) -> TreeCursor<'a> {
        self.tree.cursor()
    }

    /// apply an edit by replacing text
    // TODO: wasm compat
    pub fn edit(&mut self, delete: Span, insert: &str) -> EditResult {
        // apply string edit
        // PERF: i may want to use a rope or something that handles edits better
        let mut new_source = self.tree.source().to_string();
        new_source.replace_range(delete.start as usize..delete.end as usize, insert);

        let delta = insert.len() as isize - (delete.end - delete.start) as isize;
        let cache = Cache::new(&self.tree, delete, delta);

        let mut ctx = ParseContext::new(&new_source, Some(cache));
        self.tree = ctx.parse_document();

        EditResult {}
    }
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl Parsed {
    /// get the syntax tree
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "tree"))]
    pub fn js_tree(&self) -> JsValue {
        todo!()
    }
}

impl<'a> ParseContext<'a> {
    pub fn new(source: &'a str, cache: Option<Cache<'a>>) -> Self {
        Self {
            builder: TreeBuilder::new(source.to_string()),
            tokenizer: Tokenizer::new(source),
            cache,
            pos: 0,
        }
    }

    pub fn parse_document(mut self) -> Tree {
        let doc_span = Span {
            start: 0,
            end: self.builder.source().len() as Len,
        };
        let root = self.builder.push_node(NodeKind::Document, doc_span);

        // TODO: implement actual parsing here
        while let Some(token) = self.tokenizer.advance() {
            // For now, emit everything as Text nodes attached to a dummy Paragraph if not blocks.
            // A full implementation would check block boundaries, lists, headers, etc.
            let text_span = token.span;

            // To simulate incremental checking:
            if let Some(ref cache) = self.cache {
                if let Some(reused) = cache.find_reusable_block(text_span.start) {
                    // Shift subtree logic here
                    continue;
                }
            }

            let text_node = self
                .builder
                .push_node(NodeKind::Text(TextKind::Text), text_span);
            self.builder.add_child(root, text_node);
        }

        self.builder.build()
    }
}
