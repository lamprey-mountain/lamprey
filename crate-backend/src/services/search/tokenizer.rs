use std::sync::Mutex;

use once_cell::sync::Lazy;
use tantivy::tokenizer::{
    BoxTokenStream, Language, LowerCaser, RemoveLongFilter, SimpleTokenizer, Stemmer, TextAnalyzer,
    Tokenizer,
};

static ANALYZER: Lazy<TextAnalyzer> = Lazy::new(|| {
    TextAnalyzer::builder(SimpleTokenizer::default())
        .filter(RemoveLongFilter::limit(40))
        .filter(LowerCaser)
        .filter(Stemmer::new(Language::English))
        .build()
});

/// a tokenizer that can change depending on the language
#[derive(Clone)]
pub struct DynamicTokenizer {
    analyzer: TextAnalyzer,
}

impl DynamicTokenizer {
    pub fn new() -> Self {
        Self {
            analyzer: ANALYZER.clone(),
        }
    }
}

impl Tokenizer for DynamicTokenizer {
    type TokenStream<'a> = BoxTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        self.analyzer.token_stream(text)
    }
}
