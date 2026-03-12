use lamprey_common::v1::types::EmojiId;

use crate::ast::Ast;
use crate::events::{Event, EventFilter};
use crate::render::MarkdownReader;

/// A reader that filters out disallowed custom emoji.
///
/// This reader filters out custom emoji (`<:name:uuid>` or `<a:name:uuid>`) that are not in the
/// allowed list. All other text content is preserved.
///
/// # Example
/// ```
/// use lamprey_markdown::{Parser, Ast, StripEmojiReader};
/// use lamprey_common::v1::types::EmojiId;
/// use uuid::uuid;
///
/// let allowed = vec![EmojiId::from(uuid!("12345678-1234-1234-1234-123456789abc"))];
/// let parser = Parser::default();
/// let parsed = parser.parse("hello <:smile:12345678-1234-1234-1234-123456789abc> world");
/// let ast = Ast::new(parsed);
/// let reader = StripEmojiReader::new(allowed);
///
/// let result = reader.read(&ast);
/// assert!(result.contains("smile"));
/// ```
pub struct StripEmojiReader {
    pub allowed: Vec<EmojiId>,
}

impl StripEmojiReader {
    /// Create a new StripEmojiReader with the allowed emoji list.
    pub fn new(allowed: Vec<EmojiId>) -> Self {
        StripEmojiReader { allowed }
    }

    /// Read text from an AST, filtering out disallowed emoji.
    pub fn read(&self, ast: &Ast) -> String {
        // For now, just strip all emoji
        // TODO: Implement proper filtering based on allowed list
        ast.events()
            .strip_emoji()
            .map(|event| event.text())
            .collect()
    }
}

impl MarkdownReader for StripEmojiReader {
    fn read(&self, ast: &Ast) -> String {
        self.read(ast)
    }
}
