use crate::lexer::TokenKind;
use logos::Logos;

#[test]
fn test_tokenization() {
    let source = "~~strike~~ ||spoiler|| <@123e4567-e89b-12d3-a456-426614174000> <&uuid> <#uuid> <:name:uuid> - [x] - [ ] |";
    let mut lexer = TokenKind::lexer(source);

    assert_eq!(lexer.next(), Some(Ok(TokenKind::Tilde2)));
    assert_eq!(lexer.next(), Some(Ok(TokenKind::Text))); // "strike"
    assert_eq!(lexer.next(), Some(Ok(TokenKind::Tilde2)));
    assert_eq!(lexer.next(), Some(Ok(TokenKind::Whitespace)));
    assert_eq!(lexer.next(), Some(Ok(TokenKind::Pipe2)));
    assert_eq!(lexer.next(), Some(Ok(TokenKind::Text))); // "spoiler"
    assert_eq!(lexer.next(), Some(Ok(TokenKind::Pipe2)));
    assert_eq!(lexer.next(), Some(Ok(TokenKind::Whitespace)));
    assert_eq!(lexer.next(), Some(Ok(TokenKind::AngleOpen)));
    assert_eq!(lexer.next(), Some(Ok(TokenKind::At)));
    assert_eq!(lexer.next(), Some(Ok(TokenKind::Uuid)));
    assert_eq!(lexer.next(), Some(Ok(TokenKind::AngleClose)));
    assert_eq!(lexer.next(), Some(Ok(TokenKind::Whitespace)));
    assert_eq!(lexer.next(), Some(Ok(TokenKind::AngleOpen)));
    assert_eq!(lexer.next(), Some(Ok(TokenKind::Ampersand)));
    assert_eq!(lexer.next(), Some(Ok(TokenKind::Text))); // "uuid"
    assert_eq!(lexer.next(), Some(Ok(TokenKind::AngleClose)));
}
