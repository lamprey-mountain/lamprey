use rowan::{GreenNodeBuilder, NodeCache};

#[cfg(feature = "serde")]
use crate::ast::serialized::SerializedDocument;
use crate::lexer::{Lexer, Source};
use crate::parser::config::ParserConfig;
use crate::prelude::*;
use crate::transform::Transform;

mod block;
pub mod config;
mod inline;

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
    tree: Ref<Tree>,
    cache: NodeCache,
    source: Source,
}

// TODO: add doc comment
// TODO: merge into Parsed?
pub struct ParseContext<'a> {
    builder: GreenNodeBuilder<'a>,
    tokenizer: Lexer<'a>,
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
        let source = Source::new(markdown);
        let mut cache = NodeCache::default();
        let ctx = ParseContext::new(&source, &mut cache);
        let tree = ctx.parse_document();
        Parsed {
            config,
            tree: Ref::new(tree),
            cache,
            source,
        }
    }
}

impl Parsed {
    /// get the syntax tree
    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    /// get a cloned ref to the syntax tree
    pub fn tree_clone(&self) -> Ref<Tree> {
        Ref::clone(&self.tree)
    }

    /// apply an edit by replacing text
    pub fn edit(&mut self, delete: Span, insert: &str) {
        self.source.edit(delete, insert);
        let ctx = ParseContext::new(&self.source, &mut self.cache);
        self.tree = Ref::new(ctx.parse_document());
    }

    /// Apply a transformation to the parsed document.
    pub fn transform<T: Transform>(&self, transformer: &T) -> Self {
        let new_green = transformer.apply(self.tree.root());
        let new_syntax_root = SyntaxNode::new_root(new_green.clone());

        // PERF: don't create source until needed
        let new_source_text = new_syntax_root.to_string();

        // PERF: reuse cache
        Self {
            config: self.config.clone(),
            tree: Ref::new(Tree { root: new_green }),
            cache: NodeCache::default(),
            source: Source::new(&new_source_text),
        }
    }

    /// get the serialized syntax tree
    #[cfg(feature = "serde")]
    pub fn ast(&self) -> SerializedDocument {
        use crate::ast::block::Document;
        use crate::ast::serialized::SerializedDocument;

        let doc = Document::cast(self.tree.root()).expect("root is document");
        let ast = SerializedDocument::from_document(doc);
        ast
    }
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl Parsed {
    /// get the serialized syntax tree
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "ast"))]
    pub fn js_ast(&self) -> JsValue {
        use crate::ast::block::Document;
        use crate::ast::serialized::SerializedDocument;

        let doc = Document::cast(self.tree.root()).expect("root is document");
        let ast = SerializedDocument::from_document(doc);
        serde_wasm_bindgen::to_value(&ast).expect("always serializable")
    }

    /// apply an edit by replacing text
    ///
    /// delete the text between `delete_start`..`delete_end` and insert text `insert`
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "edit"))]
    pub fn js_edit(&mut self, delete_start: Len, delete_end: Len, insert: &str) {
        let span = Span::from((delete_start, delete_end));
        self.edit(span, insert);
    }
}

impl<'a> ParseContext<'a> {
    pub fn new(source: &'a Source, cache: &'a mut NodeCache) -> Self {
        Self {
            builder: GreenNodeBuilder::with_cache(cache),
            tokenizer: Lexer::new(&source.0),
        }
    }
}
