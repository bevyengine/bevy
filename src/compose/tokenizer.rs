use std::collections::VecDeque;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Token<'a> {
    Identifier(&'a str, usize),
    Other(char, usize),
    Whitespace(&'a str, usize),
}

impl<'a> Token<'a> {
    pub fn pos(&self) -> usize {
        match self {
            Token::Identifier(_, pos) | Token::Other(_, pos) | Token::Whitespace(_, pos) => *pos,
        }
    }

    pub fn identifier(&self) -> Option<&str> {
        match self {
            Token::Identifier(ident, _) => Some(ident),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Identifier,
    Whitespace,
}

// a basic tokenizer that separates identifiers from non-identifiers, and optionally returns whitespace tokens
// unicode XID rules apply, except that additional characters '"' and '::' (sequences of two colons) are allowed in identifiers.
// quotes treat any further chars until the next quote as part of the identifier.
// note we don't support non-USV identifiers like 👩‍👩‍👧‍👧 which is apparently in XID_continue
pub struct Tokenizer<'a> {
    tokens: VecDeque<Token<'a>>,
}

impl<'a> Tokenizer<'a> {
    pub fn new(src: &'a str, emit_whitespace: bool) -> Self {
        let mut tokens = VecDeque::default();
        let mut current_token_start = 0;
        let mut current_token = None;
        let mut quoted_token = false;

        let mut chars = src.char_indices().peekable();

        while let Some((ix, char)) = chars.next() {
            if char == '"' {
                quoted_token = !quoted_token;
                if !quoted_token {
                    continue;
                }
            }

            if let Some(tok) = current_token {
                match tok {
                    TokenKind::Identifier => {
                        // accept anything within quotes, or XID_continues
                        if quoted_token || unicode_ident::is_xid_continue(char) {
                            continue;
                        }
                        // accept `::`
                        if char == ':' && chars.peek() == Some(&(ix + 1, ':')) {
                            chars.next();
                            continue;
                        }

                        tokens.push_back(Token::Identifier(
                            &src[current_token_start..ix],
                            current_token_start,
                        ));
                    }
                    TokenKind::Whitespace => {
                        if char.is_whitespace() {
                            continue;
                        }
                        tokens.push_back(Token::Whitespace(
                            &src[current_token_start..ix],
                            current_token_start,
                        ));
                    }
                };

                current_token_start = ix;
                current_token = None;
            }

            if quoted_token || unicode_ident::is_xid_start(char) {
                current_token = Some(TokenKind::Identifier);
                current_token_start = ix;
            } else if !char.is_whitespace() {
                tokens.push_back(Token::Other(char, ix));
            } else if char.is_whitespace() && emit_whitespace {
                current_token = Some(TokenKind::Whitespace);
                current_token_start = ix;
            }
        }

        if let Some(tok) = current_token {
            match tok {
                TokenKind::Identifier => {
                    tokens.push_back(Token::Identifier(
                        &src[current_token_start..src.len()],
                        current_token_start,
                    ));
                }
                TokenKind::Whitespace => {
                    tokens.push_back(Token::Whitespace(
                        &src[current_token_start..src.len()],
                        current_token_start,
                    ));
                }
            };
        }

        Self { tokens }
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.tokens.pop_front()
    }
}
