use crate::grammar::Table;
use crate::parser::config::ParserConfig;
use crate::prelude::*;

use crate::tree::cursor::TreeCursor;
use crate::tree::Tree;

pub mod config;

/// a markdown parser
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct Parser {
    /// core glr state machine definition
    table: Ref<Table>,
}

/// parsed markdown
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct Parsed {
    config: ParserConfig,
    table: Ref<Table>,
    tree: Tree,
}

/// the result of an edit
pub struct EditResult {
    // TODO: maybe have added/removed Decorations?
}

/// single parse state
pub struct Stack {
    state_id: u32,
    // TODO: symbol stacks, lookaheads, etc.
}

pub struct ParseContext {
    /// all current glr branches
    ///
    /// contains one item if unambiguous
    stacks: Vec<Stack>,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Parser {
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new() -> Self {
        todo!()
    }

    /// parse some markdown
    pub fn parse(&self, markdown: &str) -> Parsed {
        Parsed::new(Ref::clone(&self.table), markdown)
    }

    /// parse some markdown with config
    pub fn parse_with_config(&self, markdown: &str, config: ParserConfig) -> Parsed {
        todo!()
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Parsed {
    fn new(table: Ref<Table>, source: &str) -> Self {
        todo!()
    }

    /// get the source string
    pub fn source(&self) -> String {
        todo!()
    }
}

impl Parsed {
    /// get the syntax tree
    pub fn tree(&self) -> &Tree {
        todo!()
    }

    /// get a cursor for the syntax tree
    // TODO: wasm compat
    pub fn cursor<'a>(&'a self) -> TreeCursor<'a> {
        todo!()
    }

    /// apply an edit by replacing text
    // TODO: wasm compat
    pub fn edit(&mut self, delete: Span, insert: &str) -> EditResult {
        // Modify your parse loop to check the old tree cache before running the
        // tokenizer. If a node exists at the current position that was unaffected by the
        // edit, fast-forward the parser state and graft the old subtree directly into the
        // new one.

        // Step 1: Find the initial block-level region to reparse
        // Step 2: Iteratively expand and reparse until boundaries stabilize

        todo!()
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

impl ParseContext {
    /// advance the parser by a single step
    ///
    /// returns `Some(Tree)` when parsing is fully complete
    pub fn advance(&mut self) -> Option<Tree> {
        // Process tokens, split stacks if ambiguous, discard failing branches
        // let current_state = self.stacks[0].state_id;

        // // 1. Ask the parse tables what token IDs are legal right now
        // let legal_mask = self.parser.tables.get_expected_tokens(current_state);

        // // 2. TOKENIZE: Read exactly ONE token under the legal guidelines
        // let token = self.tokenizer.next_token(&self.input, self.pos, legal_mask);

        // // 3. PARSE: Evaluate lookups against the LR Action tables
        // match self.parser.tables.get_action(current_state, token.term_id) {
        //     ParseAction::Shift { next_state } => {
        //         self.stacks[0].push(next_state, token);
        //         self.pos = token.end; // Move stream pointer forward
        //     }
        //     ParseAction::Reduce { rule_id } => {
        //         self.reduce_nodes(rule_id);
        //         // Do not increment self.pos; re-evaluate this token in the new state context
        //     }
        //     ParseAction::Accept => return Some(self.build_tree()),
        //     ParseAction::Error => self.run_error_recovery(),
        // }

        // None
        todo!()
    }

    // /// Executed inside the main `advance()` loop.
    // fn get_next_action(&mut self) -> ParseAction {
    //     // 1. TRY INCREMENTAL REUSE:
    //     // Check if the old tree has a perfectly valid, untouched subtree right here.
    //     if let Some(old_node) = self.cache.find_reusable_node(self.pos, self.current_state) {
    //         // Success! We can skip parsing this entire language construct.
    //         self.pos = old_node.end; // Fast-forward the stream pointer past the node
    //         return ParseAction::ShiftSubtree(old_node);
    //     }

    //     // 2. FALLBACK TO STANDARD PARSING:
    //     // If no reusable node is found, fall back to normal tokenizing and shifting.
    //     let token = self.lexer.next_token(&self.input, self.pos);
    //     self.compute_lr_action(token)
    // }
}
