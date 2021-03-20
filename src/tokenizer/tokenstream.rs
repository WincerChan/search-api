use tantivy::tokenizer::{Token, TokenStream};

pub struct UTF8TokenStream {
    pub tokens: Vec<Token>,
    pub offset: usize,
}

impl TokenStream for UTF8TokenStream {
    fn advance(&mut self) -> bool {
        if self.offset < self.tokens.len() {
            self.offset += 1;
            true
        } else {
            false
        }
    }

    fn token(&self) -> &Token {
        &self.tokens[self.offset - 1]
    }

    fn token_mut(&mut self) -> &mut Token {
        &mut self.tokens[self.offset - 1]
    }
}
