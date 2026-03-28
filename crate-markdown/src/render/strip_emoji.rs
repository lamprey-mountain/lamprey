use lamprey_common::v1::types::EmojiId;
use uuid::Uuid;

use crate::ast::{Ast, AstNode, Emoji};
use crate::render::MarkdownReader;

/// A reader that filters out disallowed custom emoji while preserving all other markdown formatting.
///
/// This reader filters out custom emoji (`<:name:uuid>` or `<a:name:uuid>`) that are not in the
/// allowed list. Disallowed emoji are converted to `:name:` format (name only, no UUID).
/// Allowed emoji are preserved in their original `<:name:uuid>` or `<a:name:uuid>` format.
///
/// All other markdown formatting (bold, italic, blockquotes, headings, mentions, links, etc.)
/// is preserved exactly as in the original source.
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

    /// Read markdown from an AST, filtering out disallowed emoji while preserving all other formatting.
    ///
    /// This method uses the AST to identify emoji nodes and builds the output by replacing only
    /// those specific emoji strings. All other markdown (bold, italic, headings, blockquotes,
    /// lists, links, mentions, code blocks, inline code, etc.) is preserved byte-for-byte.
    ///
    /// This approach correctly handles emoji in code blocks and inline code - they are NOT stripped
    /// because the AST correctly identifies them as code content, not emoji nodes.
    ///
    /// Allowed emoji are preserved as `<:name:uuid>` or `<a:name:uuid>`.
    /// Disallowed emoji are converted to `:name:` format.
    pub fn read(&self, ast: &Ast) -> String {
        let source = ast.source();
        let mut result = source.to_string();

        // Collect all emoji that need to be replaced (disallowed only)
        let replacements: Vec<_> = ast
            .syntax()
            .descendants()
            .filter_map(|node| Emoji::cast(node.clone()))
            .filter_map(|emoji| {
                let name = emoji.name();
                let uuid = emoji.uuid();

                // Check if this emoji is allowed
                let is_allowed = Uuid::parse_str(&uuid)
                    .ok()
                    .map(EmojiId::from)
                    .map(|id| {
                        let result = self.allowed.contains(&id);
                        result
                    })
                    .unwrap_or(false);

                if is_allowed {
                    None // Skip allowed emoji
                } else {
                    // Get the full emoji text from the syntax node
                    let emoji_text = emoji.syntax_node().text().to_string();
                    let name_str = name.to_string();
                    let replacement = format!(":{}:", name_str);
                    Some((emoji_text, replacement))
                }
            })
            .collect();

        // Apply replacements
        for (original, replacement) in replacements {
            result = result.replace(&original, &replacement);
        }

        result
    }
}

impl MarkdownReader for StripEmojiReader {
    fn read(&self, ast: &Ast) -> String {
        self.read(ast)
    }
}
