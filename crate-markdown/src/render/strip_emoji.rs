use lamprey_common::v1::types::EmojiId;
use uuid::Uuid;

use crate::ast::Ast;
use crate::events::{Event, EventFilter};
use crate::render::MarkdownReader;

/// A reader that filters out disallowed custom emoji.
///
/// This reader filters out custom emoji (`<:name:uuid>` or `<a:name:uuid>`) that are not in the
/// allowed list. Disallowed emoji are converted to `:name:` format (name only, no UUID).
/// Allowed emoji are preserved in their original `<:name:uuid>` format.
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
    ///
    /// Allowed emoji are preserved as `<:name:uuid>`.
    /// Disallowed emoji are converted to `:name:` format.
    pub fn read(&self, ast: &Ast) -> String {
        let mut result = String::new();
        let mut current_emoji_name: Option<String> = None;
        let mut current_emoji_uuid: Option<String> = None;
        let mut current_emoji_animated: bool = false;

        for event in ast.events() {
            match event {
                Event::Start(crate::events::Tag::Emoji {
                    animated,
                    name,
                    uuid,
                }) => {
                    current_emoji_name = Some(name.to_string());
                    current_emoji_uuid = Some(uuid.to_string());
                    current_emoji_animated = animated;
                }
                Event::End(crate::events::Tag::Emoji {
                    animated,
                    name,
                    uuid,
                }) => {
                    // Process the emoji we just collected
                    if let (Some(name_str), Some(uuid_str)) =
                        (current_emoji_name.take(), current_emoji_uuid.take())
                    {
                        // Check if this emoji is allowed
                        let is_allowed = Uuid::parse_str(&uuid_str)
                            .ok()
                            .map(|uuid| EmojiId::from(uuid))
                            .map(|id| self.allowed.contains(&id))
                            .unwrap_or(false);

                        if is_allowed {
                            // Preserve original format <:name:uuid> or <a:name:uuid>
                            let prefix = if animated { "a" } else { "" };
                            result.push_str(&format!("<{}:{}:{}>", prefix, name_str, uuid_str));
                        } else {
                            // Convert to :name: format
                            result.push_str(&format!(":{}:", name_str));
                        }
                    }
                }
                Event::Text(text) => {
                    // Only output text if we're not inside an emoji
                    // (emoji text is handled by Start/End events)
                    if current_emoji_name.is_none() {
                        result.push_str(&text);
                    }
                }
                Event::Code(code) => {
                    result.push_str(&code);
                }
                // Skip Start/End tags for other elements
                _ => {}
            }
        }

        result
    }
}

impl MarkdownReader for StripEmojiReader {
    fn read(&self, ast: &Ast) -> String {
        self.read(ast)
    }
}
