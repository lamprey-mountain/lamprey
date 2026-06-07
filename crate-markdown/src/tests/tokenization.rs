use crate::tokenizer::TokenKind;
use logos::Logos;

#[test]
fn test_tokenization() {
    let source = "~~strike~~ ||spoiler|| <@123e4567-e89b-12d3-a456-426614174000> <&uuid> <#uuid> <:name:uuid> - [x] - [ ] |";
    let mut lexer = TokenKind::lexer(source);

    // FIXME

    dbg!(lexer.next());
    dbg!(lexer.next());
    dbg!(lexer.next());
    dbg!(lexer.next());

    // assert_eq!(lexer.next(), Some(Ok(TokenKind::Strikethrough)));
    // assert_eq!(lexer.next(), Some(Ok(TokenKind::Text))); // "strike"
    // assert_eq!(lexer.next(), Some(Ok(TokenKind::Strikethrough)));
    // assert_eq!(lexer.next(), Some(Ok(TokenKind::Whitespace)));
    // assert_eq!(lexer.next(), Some(Ok(TokenKind::Spoiler)));
    // assert_eq!(lexer.next(), Some(Ok(TokenKind::Text))); // "spoiler"
    // assert_eq!(lexer.next(), Some(Ok(TokenKind::Spoiler)));
    // assert_eq!(lexer.next(), Some(Ok(TokenKind::Whitespace)));
    // assert_eq!(lexer.next(), Some(Ok(TokenKind::UserMention)));
    // assert_eq!(lexer.next(), Some(Ok(TokenKind::Uuid)));
    // assert_eq!(lexer.next(), Some(Ok(TokenKind::Text))); // "> <&uuid> <#uuid> <:name:uuid> - [x] - [ ] |" -- wait, this needs better handling
}
