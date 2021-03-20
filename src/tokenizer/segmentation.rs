use tantivy::tokenizer::{BoxTokenStream, Token, Tokenizer};

use super::tokenstream::UTF8TokenStream;

#[derive(Clone)]
pub struct UTF8Tokenizer;

pub fn cut_string<'a>(text: &'a str) -> Vec<&'a str> {
    let mut char_offset = 0usize;
    let mut byte_offset = 0usize;
    let words: Vec<char> = text.chars().collect();
    let mut result: Vec<&str> = Vec::new();
    while char_offset < words.len() {
        let mut byte_start = byte_offset;
        let mut char_start = char_offset;
        let mut word = words[char_start];
        if !word.is_ascii_alphabetic() {
            byte_start += word.len_utf8();
            char_start += 1;
        }
        while word.is_ascii_alphabetic() {
            if char_start == words.len() - 1 {
                byte_start += word.len_utf8();
                char_start += 1;
                break;
            }
            char_start += 1;
            byte_start += word.len_utf8();
            word = words[char_start];
        }
        result.push(&text[byte_offset..byte_start]);
        byte_offset = byte_start;
        char_offset = char_start;
    }
    result
}

impl Tokenizer for UTF8Tokenizer {
    fn token_stream<'a>(&self, text: &'a str) -> BoxTokenStream<'a> {
        let words = cut_string(text);
        let mut offset = 0usize;
        let mut tokens = Vec::with_capacity(words.len());
        for word in words {
            let next = offset + word.len();
            let token = Token {
                offset_from: offset,
                offset_to: offset + word.len(),
                position: offset,
                text: word.to_lowercase(),
                position_length: word.len(),
            };
            offset = next;
            tokens.push(token);
        }
        BoxTokenStream::from(UTF8TokenStream { tokens, offset: 0 })
    }
}
