use logos::Logos;
use rowan::{GreenNode, GreenNodeBuilder, NodeOrToken, TextRange};
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

/// Parsing context that holds state during parsing.
/// This centralizes parser state and removes parameter clutter from parsing functions.
struct ParseContext<'a> {
    builder: GreenNodeBuilder<'static>,
    source: &'a str,
    tokens: &'a [(Result<TokenKind, ()>, Range<usize>)],
    pos: usize,
    inline_mapping: Option<&'a HashMap<usize, usize>>,
}

impl<'a> ParseContext<'a> {
    /// Create a new parsing context.
    fn new(
        source: &'a str,
        tokens: &'a [(Result<TokenKind, ()>, Range<usize>)],
        inline_mapping: Option<&'a HashMap<usize, usize>>,
    ) -> Self {
        Self {
            builder: GreenNodeBuilder::new(),
            source,
            tokens,
            pos: 0,
            inline_mapping,
        }
    }

    /// Peek at the current token kind without advancing.
    fn peek(&self) -> Option<TokenKind> {
        self.tokens
            .get(self.pos)
            .and_then(|(res, _)| res.as_ref().ok())
            .copied()
    }

    /// Check if the current token matches the given kind.
    fn at(&self, kind: TokenKind) -> bool {
        self.peek() == Some(kind)
    }

    /// Advance to the next token.
    fn bump(&mut self) {
        self.pos += 1;
    }

    /// Advance and consume the current token if it matches the expected kind.
    /// Returns true if the token was consumed.
    fn eat(&mut self, kind: TokenKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Get the text range of the current token.
    fn current_range(&self) -> Option<Range<usize>> {
        self.tokens
            .get(self.pos)
            .and_then(|(_, range)| Some(range.clone()))
    }

    /// Get the text of the current token.
    fn current_text(&self) -> Option<&'a str> {
        let range = self.current_range()?;
        self.source.get(range.start..range.end)
    }

    /// Get text for a given range.
    fn text_for_range(&self, range: Range<usize>) -> &'a str {
        &self.source[range.start..range.end]
    }

    /// Check if we're at or past the end of tokens.
    fn is_eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    /// Get current token position.
    fn pos(&self) -> usize {
        self.pos
    }

    /// Set current token position.
    fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// Get the token at a specific position.
    fn token_at(&self, pos: usize) -> Option<TokenKind> {
        self.tokens
            .get(pos)
            .and_then(|(res, _)| res.as_ref().ok())
            .copied()
    }

    /// Get the range at a specific position.
    fn range_at(&self, pos: usize) -> Option<Range<usize>> {
        self.tokens
            .get(pos)
            .and_then(|(_, range)| Some(range.clone()))
    }

    /// Check if token at position matches kind.
    fn token_at_is(&self, pos: usize, kind: TokenKind) -> bool {
        self.token_at(pos) == Some(kind)
    }
}

// ============ ParseContext parsing methods ============

impl<'a> ParseContext<'a> {
    /// Parse inline content between delimiters, handling nested emphasis.
    /// Returns the new position after parsing.
    fn parse_inline(&mut self, end: usize) -> usize {
        while self.pos < end {
            let Some(tok) = self.peek() else {
                self.bump();
                continue;
            };
            let Some(range) = self.current_range() else {
                self.bump();
                continue;
            };

            match tok {
                TokenKind::Strong => {
                    if let Some(close_idx) = self.find_closing_delimiter(TokenKind::Strong) {
                        if close_idx <= end {
                            self.builder.start_node(SyntaxKind::Strong.into());
                            self.builder.token(SyntaxKind::StrongDelimiter.into(), "**");
                            self.bump();
                            self.parse_inline(close_idx);
                            self.builder.token(SyntaxKind::StrongDelimiter.into(), "**");
                            self.builder.finish_node();
                            self.bump();
                            continue;
                        }
                    }
                    self.builder
                        .token(SyntaxKind::Text.into(), self.text_for_range(range));
                }
                TokenKind::Emphasis => {
                    if let Some(close_idx) = self.find_closing_emphasis() {
                        if close_idx <= end {
                            self.builder.start_node(SyntaxKind::Emphasis.into());
                            self.builder
                                .token(SyntaxKind::EmphasisDelimiter.into(), "*");
                            self.bump();
                            self.parse_inline(close_idx);
                            self.builder
                                .token(SyntaxKind::EmphasisDelimiter.into(), "*");
                            self.builder.finish_node();
                            self.bump();
                            continue;
                        }
                    }
                    self.builder
                        .token(SyntaxKind::Text.into(), self.text_for_range(range));
                }
                TokenKind::Strikethrough => {
                    if let Some(close_idx) = self.find_closing_delimiter(TokenKind::Strikethrough) {
                        if close_idx <= end {
                            self.builder.start_node(SyntaxKind::Strikethrough.into());
                            self.builder
                                .token(SyntaxKind::StrikethroughDelimiter.into(), "~~");
                            self.bump();
                            self.parse_inline(close_idx);
                            self.builder
                                .token(SyntaxKind::StrikethroughDelimiter.into(), "~~");
                            self.builder.finish_node();
                            self.bump();
                            continue;
                        }
                    }
                    self.builder.token(SyntaxKind::Text.into(), "~~");
                }
                TokenKind::Backtick => {
                    // Count backticks for inline code
                    let mut fence_len = 1;
                    let j = self.pos + 1;
                    while j < end
                        && j < self.tokens.len()
                        && self.token_at_is(j, TokenKind::Backtick)
                    {
                        fence_len += 1;
                    }
                    // Find closing backticks
                    if let Some(close_idx) = self.find_closing_backticks(fence_len) {
                        if close_idx <= end {
                            self.builder.start_node(SyntaxKind::InlineCode.into());
                            self.builder.start_node(SyntaxKind::InlineCodeFence.into());
                            for _ in 0..fence_len {
                                self.builder.token(SyntaxKind::Text.into(), "`");
                            }
                            self.builder.finish_node();
                            self.pos += fence_len;
                            // Add code content
                            self.builder
                                .start_node(SyntaxKind::InlineCodeContent.into());
                            while self.pos < close_idx {
                                if let Some(range) = self.current_range() {
                                    self.builder
                                        .token(SyntaxKind::Text.into(), self.text_for_range(range));
                                }
                                self.bump();
                            }
                            self.builder.finish_node();
                            // Closing fence
                            self.builder.start_node(SyntaxKind::InlineCodeFence.into());
                            for _ in 0..fence_len {
                                self.builder.token(SyntaxKind::Text.into(), "`");
                            }
                            self.builder.finish_node();
                            self.builder.finish_node(); // InlineCode
                            self.pos += fence_len;
                            continue;
                        }
                    }
                    self.builder.token(SyntaxKind::Text.into(), "`");
                }
                TokenKind::Escape => {
                    // Handle escape sequence: \ followed by next character
                    self.builder.start_node(SyntaxKind::Escape.into());
                    self.builder.token(SyntaxKind::Text.into(), "\\");
                    self.bump();
                    // Include the escaped character
                    if self.pos < end && !self.is_eof() {
                        if let Some(range) = self.current_range() {
                            self.builder
                                .token(SyntaxKind::EscapedChar.into(), self.text_for_range(range));
                        }
                        self.bump();
                    }
                    self.builder.finish_node(); // Escape
                    continue;
                }
                TokenKind::At => {
                    // Check for mention <@uuid>
                    if self.pos + 1 < end && self.token_at_is(self.pos + 1, TokenKind::Uuid) {
                        self.builder.start_node(SyntaxKind::Mention.into());
                        self.builder.token(SyntaxKind::MentionMarker.into(), "@");
                        if let Some(range) = self.range_at(self.pos + 1) {
                            self.builder
                                .token(SyntaxKind::Text.into(), self.text_for_range(range));
                        }
                        self.builder.finish_node();
                        self.pos += 2;
                        continue;
                    }
                    self.builder.token(SyntaxKind::Text.into(), "@");
                }
                TokenKind::Colon => {
                    // Just output the colon as text
                    self.builder.token(SyntaxKind::Text.into(), ":");
                }
                TokenKind::Url => {
                    self.builder.start_node(SyntaxKind::Autolink.into());
                    self.builder.token(
                        SyntaxKind::LinkDestination.into(),
                        self.text_for_range(range),
                    );
                    self.builder.finish_node();
                }
                TokenKind::AngleOpen => {
                    // Handle emoji <:name:uuid>, <a:name:uuid>, mentions <@uuid>, and <url> autolinks
                    // Emoji format: <:name:uuid> or <a:name:uuid>
                    if self.pos + 5 < self.tokens.len() {
                        // Check for optional 'a' (animated) prefix
                        let has_animated = self.token_at_is(self.pos + 1, TokenKind::Text)
                            && self
                                .range_at(self.pos + 1)
                                .map(|r| &self.source[r.start..r.end])
                                == Some("a");

                        // For <:name:uuid>: tokens are [<, :, name, :, uuid, >]
                        // For <a:name:uuid>: tokens are [<, a, :, name, :, uuid, >]
                        let colon_pos = if has_animated {
                            self.pos + 2
                        } else {
                            self.pos + 1
                        };
                        let name_pos = if has_animated {
                            self.pos + 3
                        } else {
                            self.pos + 2
                        };

                        // Check for <: or <a: pattern (colon immediately after < or <a)
                        let colon_after_angle = self.token_at_is(colon_pos, TokenKind::Colon);

                        if colon_after_angle {
                            // Check for name:uuid pattern followed by >
                            if self.token_at_is(name_pos, TokenKind::Text)
                                && self.token_at_is(name_pos + 1, TokenKind::Colon)
                                && self.token_at_is(name_pos + 2, TokenKind::Uuid)
                                && self.token_at_is(name_pos + 3, TokenKind::AngleClose)
                            {
                                self.builder.start_node(SyntaxKind::Emoji.into());
                                self.builder.token(SyntaxKind::EmojiMarker.into(), "<");
                                if has_animated {
                                    self.builder.token(SyntaxKind::EmojiMarker.into(), "a");
                                }
                                self.builder.token(SyntaxKind::EmojiMarker.into(), ":");
                                self.builder.start_node(SyntaxKind::EmojiName.into());
                                if let Some(range) = self.range_at(name_pos) {
                                    self.builder
                                        .token(SyntaxKind::Text.into(), self.text_for_range(range));
                                }
                                self.builder.finish_node();
                                self.builder.token(SyntaxKind::EmojiMarker.into(), ":");
                                if let Some(range) = self.range_at(name_pos + 2) {
                                    self.builder
                                        .token(SyntaxKind::Text.into(), self.text_for_range(range));
                                }
                                self.builder.token(SyntaxKind::EmojiMarker.into(), ">");
                                self.builder.finish_node();
                                self.pos = name_pos + 4; // Skip past the closing >
                                continue;
                            }
                        }
                    }

                    // Handle <@uuid> mentions
                    if self.pos + 2 < self.tokens.len()
                        && self.token_at_is(self.pos + 1, TokenKind::At)
                        && self.token_at_is(self.pos + 2, TokenKind::Uuid)
                        && self.token_at_is(self.pos + 3, TokenKind::AngleClose)
                    {
                        self.builder.start_node(SyntaxKind::Mention.into());
                        self.builder.token(SyntaxKind::MentionMarker.into(), "<");
                        self.builder.token(SyntaxKind::MentionMarker.into(), "@");
                        if let Some(range) = self.range_at(self.pos + 2) {
                            self.builder
                                .token(SyntaxKind::Text.into(), self.text_for_range(range));
                        }
                        self.builder.token(SyntaxKind::MentionMarker.into(), ">");
                        self.builder.finish_node();
                        self.pos += 4;
                        continue;
                    }

                    // Handle <@&uuid> role mentions
                    if self.pos + 3 < self.tokens.len()
                        && self.token_at_is(self.pos + 1, TokenKind::At)
                        && self.token_at_is(self.pos + 2, TokenKind::Ampersand)
                        && self.token_at_is(self.pos + 3, TokenKind::Uuid)
                        && self.token_at_is(self.pos + 4, TokenKind::AngleClose)
                    {
                        self.builder.start_node(SyntaxKind::MentionRole.into());
                        self.builder.token(SyntaxKind::MentionMarker.into(), "<");
                        self.builder.token(SyntaxKind::MentionMarker.into(), "@");
                        self.builder.token(SyntaxKind::MentionMarker.into(), "&");
                        if let Some(range) = self.range_at(self.pos + 3) {
                            self.builder
                                .token(SyntaxKind::Text.into(), self.text_for_range(range));
                        }
                        self.builder.token(SyntaxKind::MentionMarker.into(), ">");
                        self.builder.finish_node();
                        self.pos += 5;
                        continue;
                    }

                    // Handle <#uuid> channel mentions
                    if self.pos + 2 < self.tokens.len()
                        && self.token_at_is(self.pos + 1, TokenKind::Hash)
                        && self.token_at_is(self.pos + 2, TokenKind::Uuid)
                        && self.token_at_is(self.pos + 3, TokenKind::AngleClose)
                    {
                        self.builder.start_node(SyntaxKind::MentionChannel.into());
                        self.builder.token(SyntaxKind::MentionMarker.into(), "<");
                        self.builder.token(SyntaxKind::MentionMarker.into(), "#");
                        if let Some(range) = self.range_at(self.pos + 2) {
                            self.builder
                                .token(SyntaxKind::Text.into(), self.text_for_range(range));
                        }
                        self.builder.token(SyntaxKind::MentionMarker.into(), ">");
                        self.builder.finish_node();
                        self.pos += 4;
                        continue;
                    }

                    // Handle <url> angle bracket links
                    if self.token_at_is(self.pos + 1, TokenKind::Url)
                        && self.token_at_is(self.pos + 2, TokenKind::AngleClose)
                    {
                        self.builder.start_node(SyntaxKind::AngleBracketLink.into());
                        if let Some(range) = self.range_at(self.pos + 1) {
                            self.builder.token(
                                SyntaxKind::LinkDestination.into(),
                                self.text_for_range(range),
                            );
                        }
                        self.builder.finish_node();
                        self.pos += 3;
                        continue;
                    }
                    self.builder.token(SyntaxKind::Text.into(), "<");
                }
                TokenKind::LinkOpen => {
                    // Handle [text](url) links
                    if let Some(link_end) = self.find_named_link_end() {
                        if link_end.paren_close < end {
                            self.builder.start_node(SyntaxKind::Link.into());
                            self.builder.start_node(SyntaxKind::LinkText.into());
                            self.builder.token(SyntaxKind::Text.into(), "[");
                            self.bump();
                            self.parse_link_text(link_end.text_close);
                            self.builder.token(SyntaxKind::Text.into(), "]");
                            self.builder.finish_node(); // LinkText
                                                        // Add destination
                            self.builder.start_node(SyntaxKind::LinkDestination.into());
                            self.builder.token(SyntaxKind::Text.into(), "(");
                            self.bump();
                            self.parse_link_dest(link_end.paren_close);
                            self.builder.token(SyntaxKind::Text.into(), ")");
                            self.builder.finish_node(); // LinkDestination
                            self.builder.finish_node(); // Link
                            self.pos = link_end.paren_close + 1;
                            continue;
                        }
                    }
                    self.builder.token(SyntaxKind::Text.into(), "[");
                }
                _ => {
                    self.builder
                        .token(SyntaxKind::Text.into(), self.text_for_range(range));
                }
            }
            self.bump();
        }
        self.pos
    }

    /// Parse link text (can contain emphasis, etc.)
    fn parse_link_text(&mut self, end: usize) -> usize {
        while self.pos < end {
            let Some(tok) = self.peek() else {
                self.bump();
                continue;
            };
            let Some(range) = self.current_range() else {
                self.bump();
                continue;
            };

            match tok {
                TokenKind::Strong => {
                    if let Some(close_idx) = self.find_closing_delimiter(TokenKind::Strong) {
                        if close_idx <= end {
                            self.builder.start_node(SyntaxKind::Strong.into());
                            self.builder.token(SyntaxKind::StrongDelimiter.into(), "**");
                            self.bump();
                            self.parse_link_text(close_idx);
                            self.builder.token(SyntaxKind::StrongDelimiter.into(), "**");
                            self.builder.finish_node();
                            self.bump();
                            continue;
                        }
                    }
                    self.builder
                        .token(SyntaxKind::Text.into(), self.text_for_range(range));
                }
                TokenKind::Emphasis => {
                    if let Some(close_idx) = self.find_closing_emphasis() {
                        if close_idx <= end {
                            self.builder.start_node(SyntaxKind::Emphasis.into());
                            self.builder
                                .token(SyntaxKind::EmphasisDelimiter.into(), "*");
                            self.bump();
                            self.parse_link_text(close_idx);
                            self.builder
                                .token(SyntaxKind::EmphasisDelimiter.into(), "*");
                            self.builder.finish_node();
                            self.bump();
                            continue;
                        }
                    }
                    self.builder
                        .token(SyntaxKind::Text.into(), self.text_for_range(range));
                }
                _ => {
                    self.builder
                        .token(SyntaxKind::Text.into(), self.text_for_range(range));
                }
            }
            self.bump();
        }
        self.pos
    }

    /// Parse link destination, handling balanced parentheses
    fn parse_link_dest(&mut self, end: usize) -> usize {
        while self.pos < end {
            let Some(range) = self.current_range() else {
                self.bump();
                continue;
            };

            self.builder.token(
                SyntaxKind::LinkDestination.into(),
                self.text_for_range(range),
            );
            self.bump();
        }
        self.pos
    }

    /// Find closing backticks for inline code
    fn find_closing_backticks(&self, fence_len: usize) -> Option<usize> {
        let mut i = self.pos + fence_len; // Start search after the opening fence
        while i < self.tokens.len() {
            let mut count = 0;
            let j = i;
            while i < self.tokens.len() && self.token_at_is(i, TokenKind::Backtick) {
                count += 1;
                i += 1;
            }
            if count >= fence_len {
                return Some(j);
            }
            if count == 0 {
                i += 1;
            }
        }
        None
    }

    /// Find the index of the closing delimiter
    fn find_closing_delimiter(&self, delimiter: TokenKind) -> Option<usize> {
        let mut depth = 0;
        for (i, (token_result, _)) in self.tokens.iter().enumerate().skip(self.pos + 1) {
            if let Ok(token) = token_result {
                if *token == delimiter {
                    if depth == 0 {
                        return Some(i);
                    }
                    depth -= 1;
                }
            }
        }
        None
    }

    /// Find the index of the closing * delimiter (not **)
    fn find_closing_emphasis(&self) -> Option<usize> {
        for (i, (token_result, _)) in self.tokens.iter().enumerate().skip(self.pos + 1) {
            if let Ok(token) = token_result {
                if *token == TokenKind::Emphasis {
                    return Some(i);
                }
            }
        }
        None
    }
}

/// Result of finding a named link end
struct LinkEnd {
    /// Index of the ] that closes the link text
    text_close: usize,
    /// Index of the ) that closes the URL (handles balanced parens)
    paren_close: usize,
}

impl<'a> ParseContext<'a> {
    /// Find the end of a named link [text](url), handling balanced parens
    fn find_named_link_end(&self) -> Option<LinkEnd> {
        // Find the closing ]
        let mut text_close = None;
        for (i, (token_result, _)) in self.tokens.iter().enumerate().skip(self.pos + 1) {
            if let Ok(token) = token_result {
                if *token == TokenKind::LinkClose {
                    text_close = Some(i);
                    break;
                }
                // If we hit a newline or another [, this isn't a valid link
                if *token == TokenKind::Newline || *token == TokenKind::LinkOpen {
                    return None;
                }
            }
        }
        let text_close = text_close?;

        // Check for ( immediately after ]
        let paren_open = text_close + 1;
        if paren_open >= self.tokens.len() {
            return None;
        }
        if let Ok(token) = &self.tokens[paren_open].0 {
            if *token != TokenKind::ParenOpen {
                return None;
            }
        } else {
            return None;
        }

        // Find the closing ), handling balanced parentheses
        let mut depth = 1;
        let mut paren_close = None;
        for (i, (token_result, _)) in self.tokens.iter().enumerate().skip(paren_open + 1) {
            if let Ok(token) = token_result {
                match token {
                    TokenKind::ParenOpen => depth += 1,
                    TokenKind::ParenClose => {
                        depth -= 1;
                        if depth == 0 {
                            paren_close = Some(i);
                            break;
                        }
                    }
                    TokenKind::Newline => return None, // Newlines break links
                    _ => {}
                }
            }
        }
        let paren_close = paren_close?;

        Some(LinkEnd {
            text_close,
            paren_close,
        })
    }

    /// Find end of current line
    fn find_line_end(&self, start: usize) -> usize {
        for (i, (token, _)) in self.tokens.iter().enumerate().skip(start) {
            if let Ok(TokenKind::Newline) = token {
                return i;
            }
        }
        self.tokens.len()
    }

    /// Find start of next non-empty line
    fn find_next_line_start(&self, start: usize) -> usize {
        let mut i = start;
        while i < self.tokens.len() {
            if let Ok(token) = &self.tokens[i].0 {
                match token {
                    TokenKind::Newline | TokenKind::Whitespace => {
                        i += 1;
                        continue;
                    }
                    _ => break,
                }
            } else {
                i += 1;
            }
        }
        i
    }

    /// Parse a header line (# Header text)
    fn parse_header(&mut self) -> usize {
        let mut level = 0;

        // Count hash symbols
        while self.token_at_is(self.pos, TokenKind::Hash) {
            level += 1;
            self.bump();
        }

        // Need more tokens after hashes
        if self.is_eof() {
            return self.pos;
        }

        self.builder.start_node(SyntaxKind::Header.into());
        self.builder.start_node(SyntaxKind::HeaderMarker.into());
        for _ in 0..level {
            self.builder.token(SyntaxKind::Text.into(), "#");
        }
        self.builder.finish_node(); // HeaderMarker

        // Parse header text
        let line_end = self.find_line_end(self.pos);
        self.parse_inline(line_end);
        self.builder.finish_node(); // Header

        // Find next non-empty line
        self.find_next_line_start(line_end)
    }

    /// List type for parsing
    fn list_type_at(&self, pos: usize) -> Option<ListType> {
        if self.token_at_is(pos, TokenKind::Dash) {
            Some(ListType::Bullet)
        } else if self.token_at_is(pos, TokenKind::Text) {
            // Check for numbered list (digit followed by dot)
            if let Some(range) = self.range_at(pos) {
                let text = &self.source[range.start..range.end];
                if text.chars().all(|c| c.is_ascii_digit())
                    && self.token_at_is(pos + 1, TokenKind::Dot)
                {
                    return Some(ListType::Numbered);
                }
            }
            None
        } else {
            None
        }
    }

    /// Check if token at position starts a list item
    fn is_list_item(&self, i: usize, list_type: ListType) -> bool {
        if i >= self.tokens.len() {
            return false;
        }
        match list_type {
            ListType::Bullet => self.token_at_is(i, TokenKind::Dash),
            ListType::Numbered => {
                if let Some(range) = self.range_at(i) {
                    let text = &self.source[range.start..range.end];
                    text.chars().all(|c| c.is_ascii_digit())
                        && self.token_at_is(i + 1, TokenKind::Dot)
                } else {
                    false
                }
            }
        }
    }

    /// Parse a list (bullet or numbered)
    fn parse_list(&mut self, list_type: ListType) -> usize {
        self.builder.start_node(SyntaxKind::List.into());

        loop {
            self.builder.start_node(SyntaxKind::ListItem.into());

            // Add marker
            self.builder.start_node(SyntaxKind::ListMarker.into());
            match list_type {
                ListType::Numbered => {
                    // Numbered list - expect Text (the number) followed by Dot
                    if let Some(range) = self.current_range() {
                        self.builder
                            .token(SyntaxKind::Text.into(), self.text_for_range(range));
                        self.bump();
                    }
                    if self.eat(TokenKind::Dot) {
                        self.builder.token(SyntaxKind::Text.into(), ".");
                    }
                }
                ListType::Bullet => {
                    // Bullet list
                    if self.eat(TokenKind::Dash) {
                        self.builder.token(SyntaxKind::Text.into(), "-");
                    }
                }
            }
            self.builder.finish_node(); // ListMarker

            // Parse item content
            let line_end = self.find_line_end(self.pos);
            if self.pos < line_end {
                self.parse_inline(line_end);
            }
            self.builder.finish_node(); // ListItem

            self.pos = self.find_next_line_start(line_end);
            if self.is_eof() {
                break;
            }

            // Check if next line is also a list item
            if !self.is_list_item(self.pos, list_type) {
                break;
            }
        }

        self.builder.finish_node(); // List
        self.pos
    }

    /// Parse a blockquote (> quote text)
    fn parse_blockquote(&mut self) -> usize {
        // Check if this is actually a blockquote marker (>)
        if !self.at(TokenKind::AngleClose) {
            return self.pos;
        }

        self.builder.start_node(SyntaxKind::BlockQuote.into());

        loop {
            // Consume > marker
            if self.eat(TokenKind::AngleClose) {
                self.builder.start_node(SyntaxKind::BlockQuoteMarker.into());
                self.builder.token(SyntaxKind::Text.into(), ">");
                self.builder.finish_node(); // BlockQuoteMarker
            }

            // Parse line content
            let line_end = self.find_line_end(self.pos);
            if self.pos < line_end {
                self.parse_inline(line_end);
            }

            self.pos = self.find_next_line_start(line_end);
            if self.is_eof() || !self.at(TokenKind::AngleClose) {
                break;
            }
        }

        self.builder.finish_node(); // BlockQuote
        self.pos
    }

    /// Parse a fenced code block
    fn parse_code_block(&mut self) -> usize {
        let start_pos = self.pos;
        let mut fence_len = 0;

        // Count backticks
        while self.eat(TokenKind::Backtick) {
            fence_len += 1;
        }

        if fence_len < 3 {
            // Not a code fence, return original position
            self.pos = start_pos;
            return start_pos;
        }

        self.builder.start_node(SyntaxKind::CodeBlock.into());
        self.builder.start_node(SyntaxKind::CodeBlockFence.into());
        for _ in 0..fence_len {
            self.builder.token(SyntaxKind::Text.into(), "`");
        }
        self.builder.finish_node(); // CodeBlockFence

        // Find closing fence
        let content_start = self.pos;
        while !self.is_eof() {
            let mut close_len = 0;
            let j = self.pos;
            while self.eat(TokenKind::Backtick) {
                close_len += 1;
            }
            if close_len >= fence_len {
                // Found closing fence
                self.builder.start_node(SyntaxKind::CodeBlockContent.into());
                // Add content between fences
                let mut k = content_start;
                while k < j {
                    if let Some(range) = self.range_at(k) {
                        self.builder
                            .token(SyntaxKind::Text.into(), self.text_for_range(range));
                    }
                    k += 1;
                }
                self.builder.finish_node(); // CodeBlockContent

                self.builder.start_node(SyntaxKind::CodeBlockFence.into());
                for _ in 0..close_len {
                    self.builder.token(SyntaxKind::Text.into(), "`");
                }
                self.builder.finish_node(); // CodeBlockFence
                self.builder.finish_node(); // CodeBlock
                return self.pos;
            }
            // No closing fence found at this position, advance
            if !self.is_eof() {
                self.bump();
            }
        }

        // No closing fence, just finish what we have
        self.builder.start_node(SyntaxKind::CodeBlockContent.into());
        let mut k = content_start;
        while k < self.tokens.len() {
            if let Some(range) = self.range_at(k) {
                self.builder
                    .token(SyntaxKind::Text.into(), self.text_for_range(range));
            }
            k += 1;
        }
        self.builder.finish_node(); // CodeBlockContent
        self.builder.finish_node(); // CodeBlock
        self.pos
    }

    /// Find end of paragraph
    fn find_paragraph_end(&self, start: usize) -> usize {
        let mut i = start;
        let mut prev_was_newline = false;

        while i < self.tokens.len() {
            if let Ok(token) = &self.tokens[i].0 {
                match token {
                    TokenKind::Newline => {
                        if prev_was_newline {
                            return i - 1; // Blank line ends paragraph
                        }
                        prev_was_newline = true;
                    }
                    TokenKind::Whitespace => {}
                    TokenKind::Hash
                    | TokenKind::Dash
                    | TokenKind::AngleClose
                    | TokenKind::Backtick => {
                        // Block element starts
                        if prev_was_newline {
                            return i - 1;
                        }
                        prev_was_newline = false;
                    }
                    _ => {
                        prev_was_newline = false;
                    }
                }
            }
            i += 1;
        }
        i
    }

    /// Parse a paragraph with inline content
    fn parse_paragraph(&mut self) -> usize {
        self.builder.start_node(SyntaxKind::Paragraph.into());

        // Find end of paragraph (blank line or block element)
        let para_end = self.find_paragraph_end(self.pos);

        // Parse inline content
        self.parse_inline(para_end);

        self.builder.finish_node(); // Paragraph

        // Move past any whitespace/newlines at the end
        self.find_next_line_start(para_end)
    }

    /// Parse markdown source into block-level elements using the context
    fn parse_blocks(&mut self) {
        while !self.is_eof() {
            let Some(token) = self.peek() else {
                self.bump();
                continue;
            };

            match token {
                TokenKind::Newline | TokenKind::Whitespace => {
                    self.bump();
                }
                TokenKind::Hash => {
                    self.pos = self.parse_header();
                }
                TokenKind::Dash => {
                    self.pos = self.parse_list(ListType::Bullet);
                }
                TokenKind::Text => {
                    if let Some(range) = self.current_range() {
                        let text = self.text_for_range(range);
                        if text.chars().all(|c| c.is_ascii_digit())
                            && self.token_at_is(self.pos + 1, TokenKind::Dot)
                        {
                            self.pos = self.parse_list(ListType::Numbered);
                            continue;
                        }
                    }
                    self.pos = self.parse_paragraph();
                }
                TokenKind::AngleClose => {
                    self.pos = self.parse_blockquote();
                }
                TokenKind::Backtick => {
                    let old_pos = self.pos;
                    self.pos = self.parse_code_block();
                    if self.pos == old_pos {
                        self.pos = self.parse_paragraph();
                    }
                }
                _ => {
                    self.pos = self.parse_paragraph();
                }
            }
        }
    }
}

/// Token kinds used by logos for lexing.
#[derive(Debug, Clone, Copy, Logos, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TokenKind {
    #[regex(r"[ \t]+")]
    Whitespace,

    #[regex(r"\n+")]
    Newline,

    #[token("\\")]
    Escape,

    #[token("**")]
    Strong,

    #[token("*")]
    Emphasis,

    #[token("~~")]
    Strikethrough,

    #[token("`")]
    Backtick,

    #[token("[")]
    LinkOpen,

    #[token("]")]
    LinkClose,

    #[token("(")]
    ParenOpen,

    #[token(")")]
    ParenClose,

    #[token("<")]
    AngleOpen,

    #[token(">")]
    AngleClose,

    #[token("#")]
    Hash,

    #[token("-")]
    Dash,

    #[token(".")]
    Dot,

    #[token("@")]
    At,

    #[token("&")]
    Ampersand,

    #[token(":")]
    Colon,

    /// UUID pattern for mentions and emoji
    #[regex("[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}")]
    Uuid,

    /// URL pattern for autolinks and link destinations
    #[regex(r"https?://[^\s\]\)>]+")]
    Url,

    /// any other text (words, punctuation, etc.) - excluding special chars
    #[regex(r"[^ \t\n*\\`<>\[\]\(\)#@:~.\-&]+")]
    Text,
}

/// Syntax node kinds for rowan.
/// Fieldless enum that maps directly to u16 for use with rowan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u16)]
pub enum SyntaxKind {
    Root = 0,
    Document,
    // Block elements
    Paragraph,
    Header,
    HeaderMarker,
    List,
    ListItem,
    ListMarker,
    BlockQuote,
    BlockQuoteMarker,
    CodeBlock,
    CodeBlockFence,
    CodeBlockContent,
    // Inline elements
    Text,
    Strong,
    StrongDelimiter,
    Emphasis,
    EmphasisDelimiter,
    Strikethrough,
    StrikethroughDelimiter,
    InlineCode,
    InlineCodeFence,
    InlineCodeContent,
    Link,
    LinkText,
    LinkDestination,
    LinkTitle,
    Autolink,
    AngleBracketLink,
    Mention,
    MentionMarker,
    MentionRole,
    MentionChannel,
    Emoji,
    EmojiName,
    EmojiMarker,
    // Escape sequences
    Escape,
    EscapedChar,
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        Self(kind as u16)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MyLang {}

impl rowan::Language for MyLang {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        // SAFETY: SyntaxKind is a fieldless enum with #[repr(u16)],
        // so it can be safely transmuted from u16
        // FIXME: rewrite this; ideally there won't be any unsafe at all
        unsafe { std::mem::transmute(raw.0) }
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.into()
    }
}

pub type SyntaxNode = rowan::SyntaxNode<MyLang>;

/// Enable or disable different parse options.
#[derive(Debug, Default, Clone, Copy)]
pub struct ParseOptions {
    /// Whether to enable headers. Currently unused but reserved for future use.
    pub headers: bool,
}

/// Main parser for markdown text.
///
/// # Example
/// ```
/// use lamprey_markdown::{Parser, ParseOptions};
///
/// let parser = Parser::new(ParseOptions::default());
/// let parsed = parser.parse("**hello** world");
/// let tree = parsed.syntax();
/// ```
#[allow(dead_code)]
pub struct Parser {
    options: ParseOptions,
}

/// Result of parsing, containing the syntax tree and original source.
#[derive(Debug, Clone)]
pub struct Parsed {
    green: GreenNode,
    source: Arc<str>,
}

impl Parsed {
    /// Get the syntax tree root node.
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }

    /// Get the original source text.
    pub fn source(&self) -> &str {
        &self.source
    }
}

/// A change to apply to a parsed document for incremental editing.
#[derive(Debug, Clone)]
pub struct Edit<'a> {
    /// The span to delete (in byte offsets).
    pub delete: TextRange,
    /// The text to insert.
    pub insert: &'a str,
}

impl Parser {
    /// Create a new parser with the given options.
    pub fn new(options: ParseOptions) -> Self {
        Self { options }
    }

    /// Parse markdown source into a syntax tree.
    pub fn parse(&self, source: &str) -> Parsed {
        let green = parse(source);
        Parsed {
            green,
            source: Arc::from(source),
        }
    }

    /// Edit a parsed document incrementally, reusing unchanged portions of the old tree.
    ///
    /// # Example
    /// ```
    /// use lamprey_markdown::{Parser, Edit};
    /// use rowan::TextRange;
    ///
    /// let parser = Parser::new(Default::default());
    /// let parsed = parser.parse("**hello** world");
    /// let edit = Edit {
    ///     delete: TextRange::new(10.into(), 15.into()),
    ///     insert: "universe",
    /// };
    /// let edited = parser.edit(&parsed, edit);
    /// assert_eq!(edited.source(), "**hello** universe");
    /// ```
    pub fn edit(&self, parsed: &Parsed, edit: Edit<'_>) -> Parsed {
        let Edit { delete, insert } = edit;
        let old_source = parsed.source.as_ref();
        let old_tree = &parsed.green;

        // Calculate edit ranges
        let edit_start = usize::from(delete.start());
        let edit_end = usize::from(delete.end());
        let edit_len = delete.len().into();
        let insert_len = insert.len();

        // Build new source
        let mut new_source = String::with_capacity(old_source.len() - edit_len + insert_len);
        new_source.push_str(&old_source[..edit_start]);
        new_source.push_str(insert);
        new_source.push_str(&old_source[edit_end..]);

        // Incremental parse, reusing unchanged portions of the tree
        let green = parse_incremental(old_tree, &new_source, edit_start, edit_len, insert_len);
        Parsed {
            green,
            source: Arc::from(new_source),
        }
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new(ParseOptions::default())
    }
}

/// Parse markdown incrementally, reusing unchanged portions of the old tree.
///
/// The key insight is that Markdown is context-sensitive: changes can affect block boundaries.
/// For example, adding a space before `-` can break a list, or adding `*` can break emphasis.
///
/// This function:
/// 1. Finds the block-level element(s) affected by the edit
/// 2. Expands the invalidation region to include full blocks
/// 3. Re-parses from the start of the first affected block
/// 4. Checks if block boundaries match; if not, expands further
fn parse_incremental(
    old_tree: &GreenNode,
    new_source: &str,
    edit_start: usize,
    edit_len: usize,
    insert_len: usize,
) -> GreenNode {
    use rowan::{NodeOrToken, SyntaxNode};

    let old_root = SyntaxNode::new_root(old_tree.clone());

    // Step 1: Find the initial block-level region to reparse
    let (mut reparsed_start, mut reparsed_end, mut blocks_before, mut blocks_after) =
        find_affected_blocks(
            &old_root,
            edit_start,
            edit_start + edit_len,
            edit_len,
            insert_len,
            new_source.len(),
        );

    // Step 2: Iteratively expand and reparse until boundaries stabilize
    // This handles cascade effects where broken delimiters absorb adjacent content
    let mut iteration = 0;
    let max_iterations = 5;

    loop {
        let mut builder = GreenNodeBuilder::new();
        builder.start_node(SyntaxKind::Root.into());
        builder.start_node(SyntaxKind::Document.into());

        // Add reusable blocks before
        for (child, _start, _end) in &blocks_before {
            match child {
                NodeOrToken::Node(node) => {
                    let kind: SyntaxKind = node.kind();
                    builder.start_node(kind.into());
                    copy_subtree(&mut builder, node);
                    builder.finish_node();
                }
                NodeOrToken::Token(token) => {
                    builder.token(token.kind().into(), token.text());
                }
            }
        }

        // Reparse affected region
        let affected_source = &new_source[reparsed_start..reparsed_end];
        parse_block_region(&mut builder, affected_source, reparsed_start);

        // Add reusable blocks after
        for (child, _start, _end) in &blocks_after {
            match child {
                NodeOrToken::Node(node) => {
                    let kind: SyntaxKind = node.kind();
                    builder.start_node(kind.into());
                    copy_subtree(&mut builder, node);
                    builder.finish_node();
                }
                NodeOrToken::Token(token) => {
                    builder.token(token.kind().into(), token.text());
                }
            }
        }

        builder.finish_node(); // Document
        builder.finish_node(); // Root

        let new_tree = builder.finish();
        iteration += 1;

        // Step 3: Check if we need to expand further
        // We expand if: (1) we haven't hit max iterations, (2) there are adjacent blocks,
        // and (3) the edit was at a block boundary (potential cascade)
        if iteration >= max_iterations {
            return new_tree;
        }

        // Check if edit touched block boundaries (potential for cascade)
        let edit_at_start_boundary = blocks_before.iter().any(|(_, _, end)| *end == edit_start);
        let edit_at_end_boundary = blocks_after
            .iter()
            .any(|(_, start, _)| *start == edit_start + edit_len);

        let should_expand_start = !blocks_before.is_empty() && edit_at_start_boundary;
        let should_expand_end = !blocks_after.is_empty() && edit_at_end_boundary;

        if !should_expand_start && !should_expand_end {
            return new_tree;
        }

        // Step 4: Expand further - include more adjacent blocks
        if should_expand_start {
            if let Some(block) = blocks_before.pop() {
                reparsed_start = block.1;
            }
        }

        if should_expand_end {
            if let Some(block) = blocks_after.first() {
                reparsed_end = block.2;
                blocks_after.remove(0);
            }
        }
        // Continue loop to reparse with expanded region
    }
}

/// Find the block-level region affected by an edit.
///
/// This function implements the key fix for context-unaware incremental parsing:
/// 1. Find all blocks that overlap with the edit region
/// 2. Expand to include full blocks (not partial)
/// 3. Check if the edit could affect block boundaries (e.g., turning a list item into paragraph)
/// 4. If boundaries are ambiguous, expand further to include adjacent blocks
///
/// Returns:
/// - reparsed_start: byte offset in new source where re-parsing should begin
/// - reparsed_end: byte offset in new source where re-parsing should end
/// - blocks_before: reusable blocks before the affected region
/// - blocks_after: reusable blocks after the affected region
fn find_affected_blocks(
    old_root: &SyntaxNode,
    edit_start: usize,
    edit_end: usize,
    edit_len: usize,
    insert_len: usize,
    new_source_len: usize,
) -> (
    usize,
    usize,
    Vec<(
        NodeOrToken<SyntaxNode, rowan::SyntaxToken<MyLang>>,
        usize,
        usize,
    )>,
    Vec<(
        NodeOrToken<SyntaxNode, rowan::SyntaxToken<MyLang>>,
        usize,
        usize,
    )>,
) {
    let mut blocks_before = Vec::new();
    let mut blocks_after = Vec::new();
    let mut affected_blocks = Vec::new();

    // Collect all blocks and categorize them
    for child in old_root.children_with_tokens() {
        let start: usize = child.text_range().start().into();
        let end: usize = child.text_range().end().into();

        if end <= edit_start {
            blocks_before.push((child, start, end));
        } else if start >= edit_end {
            blocks_after.push((child, start, end));
        } else {
            // Block overlaps with edit region
            affected_blocks.push((child, start, end));
        }
    }

    // Calculate initial reparsed region based on old source
    let _old_reparsed_start = blocks_before.last().map(|(_, _, end)| *end).unwrap_or(0);
    let _old_reparsed_end = blocks_after
        .first()
        .map(|(_, start, _)| *start)
        .unwrap_or(usize::MAX);

    // Check if we need to expand the region due to context sensitivity.
    // Context-sensitive scenarios that require expansion:
    // 1. Edit at block boundary (last char of previous block or first char of next block)
    // 2. Edit that could change block type (e.g., adding space before `-`, breaking emphasis)
    // 3. Edit that affects inline delimiters which could span multiple blocks

    let mut needs_expansion = false;

    // Check if edit touches block boundaries
    if !affected_blocks.is_empty() {
        let first_affected_start = affected_blocks
            .first()
            .map(|(_, s, _)| *s)
            .unwrap_or(edit_start);
        let last_affected_end = affected_blocks
            .last()
            .map(|(_, _, e)| *e)
            .unwrap_or(edit_end);

        // If edit starts at the very beginning of a block, include the previous block
        // (e.g., user types at start of paragraph that could merge with previous)
        if edit_start == first_affected_start && !blocks_before.is_empty() {
            needs_expansion = true;
        }

        // If edit ends at the very end of a block, include the next block
        // (e.g., user types at end of paragraph that could merge with next)
        if edit_end == last_affected_end && !blocks_after.is_empty() {
            needs_expansion = true;
        }

        // Check for context-sensitive edits within paragraphs
        // Adding/removing emphasis delimiters, list markers, etc. can change structure
        for (child, _start, _end) in &affected_blocks {
            if let NodeOrToken::Node(node) = child {
                let kind = node.kind();
                // For paragraphs, inline changes can affect structure
                if kind == SyntaxKind::Paragraph {
                    // Check if edit is near emphasis-like characters
                    needs_expansion = true; // Conservative: always re-parse full paragraph
                }
                // For lists, adding space before marker can break the list
                if kind == SyntaxKind::List || kind == SyntaxKind::ListItem {
                    needs_expansion = true;
                }
            }
        }
    }

    // If expansion is needed, move blocks from before/after to affected
    let mut expanded_affected = affected_blocks;
    if needs_expansion {
        // Include previous block if it exists
        if let Some(prev) = blocks_before.pop() {
            expanded_affected.insert(0, prev);
        }
        // Include next block if it exists
        if !blocks_after.is_empty() {
            let next = blocks_after.remove(0);
            expanded_affected.push(next);
        }
    }

    // Recalculate reparsed region after expansion
    let old_reparsed_start = if let Some((_, start, _)) = expanded_affected.first() {
        *start
    } else {
        blocks_before.last().map(|(_, _, end)| *end).unwrap_or(0)
    };

    let old_reparsed_end = if let Some((_, _, end)) = expanded_affected.last() {
        *end
    } else {
        blocks_after
            .first()
            .map(|(_, start, _)| *start)
            .unwrap_or(usize::MAX)
    };

    // Map old positions to new positions
    let shift = insert_len as i32 - edit_len as i32;

    let new_reparsed_start = if old_reparsed_start <= edit_start {
        old_reparsed_start
    } else {
        (old_reparsed_start as i32 + shift) as usize
    };

    let new_reparsed_end = if old_reparsed_end == usize::MAX {
        new_source_len
    } else if old_reparsed_end >= edit_end {
        ((old_reparsed_end as i32 + shift) as usize).min(new_source_len)
    } else {
        new_source_len
    };

    (
        new_reparsed_start,
        new_reparsed_end,
        blocks_before,
        blocks_after,
    )
}

/// Parse a region of markdown source into block-level elements
fn parse_block_region(builder: &mut GreenNodeBuilder, source: &str, _region_start: usize) {
    let mut lexer = TokenKind::lexer(source).spanned();
    let tokens: Vec<_> = lexer.by_ref().collect();

    // Create a temporary ParseContext to build the region
    let mut ctx = ParseContext::new(source, &tokens, None);

    // Build blocks into the context's builder
    ctx.builder.start_node(SyntaxKind::Root.into());
    ctx.builder.start_node(SyntaxKind::Document.into());
    ctx.parse_blocks();
    ctx.builder.finish_node(); // Document
    ctx.builder.finish_node(); // Root

    // Get the temporary tree and copy its children into the outer builder
    let temp_tree = ctx.builder.finish();
    let temp_root = SyntaxNode::new_root(temp_tree);

    // Copy Document's children (the actual blocks) into the outer builder
    if let Some(doc) = temp_root
        .children()
        .find(|n| n.kind() == SyntaxKind::Document.into())
    {
        for child in doc.children_with_tokens() {
            match child {
                NodeOrToken::Node(node) => {
                    let kind: SyntaxKind = node.kind();
                    builder.start_node(kind.into());
                    copy_subtree(builder, &node);
                    builder.finish_node();
                }
                NodeOrToken::Token(token) => {
                    builder.token(token.kind().into(), token.text());
                }
            }
        }
    }
}

/// Recursively copy a subtree into the builder
fn copy_subtree(builder: &mut GreenNodeBuilder, node: &SyntaxNode) {
    use rowan::NodeOrToken;

    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Node(child_node) => {
                builder.start_node(child_node.kind().into());
                copy_subtree(builder, &child_node);
                builder.finish_node();
            }
            NodeOrToken::Token(token) => {
                builder.token(token.kind().into(), token.text());
            }
        }
    }
}

/// Parse markdown source into a GreenNode
fn parse(source: &str) -> GreenNode {
    let mut lexer = TokenKind::lexer(source).spanned();
    let tokens: Vec<_> = lexer.by_ref().collect();

    let mut ctx = ParseContext::new(source, &tokens, None);
    ctx.builder.start_node(SyntaxKind::Root.into());
    ctx.builder.start_node(SyntaxKind::Document.into());

    ctx.parse_blocks();

    ctx.builder.finish_node(); // Document
    ctx.builder.finish_node(); // Root

    ctx.builder.finish()
}

/// List type for parsing
#[derive(Clone, Copy)]
enum ListType {
    Bullet,
    Numbered,
}
