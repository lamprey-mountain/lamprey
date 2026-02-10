use std::sync::Mutex;

use once_cell::sync::Lazy;
use tantivy::tokenizer::{
    BoxTokenStream, Language, LowerCaser, RemoveLongFilter, SimpleTokenizer, Stemmer, TextAnalyzer,
    Tokenizer,
};

static ANALYZER: Lazy<Mutex<TextAnalyzer>> = Lazy::new(|| {
    Mutex::new(
        TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(RemoveLongFilter::limit(40))
            .filter(LowerCaser)
            .filter(Stemmer::new(Language::English))
            .build(),
    )
});

/// a tokenizer that can change depending on the language
#[derive(Debug, Clone)]
pub struct DynamicTokenizer;

impl DynamicTokenizer {
    pub fn new() -> Self {
        Self
    }
}

impl Tokenizer for DynamicTokenizer {
    type TokenStream<'a> = BoxTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        // HACK: wrap around the default en_stem tokenizer for now
        ANALYZER.lock().unwrap().token_stream(text)
    }
}
