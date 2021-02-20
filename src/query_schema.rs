use tantivy::{
    query::QueryParser,
    schema::{
        Field, IndexRecordOption, Schema, TextFieldIndexing, TextOptions, INDEXED, STORED, TEXT,
    },
    DocAddress, Index, IndexReader, LeasedItem, Searcher,
};

use cang_jie::{CangJieTokenizer, TokenizerOption, CANG_JIE};
use jieba_rs::Jieba;
use std::{sync::Arc, vec};

#[derive(Clone)]
struct Fields {
    url: Field,
    date: Field,
    tags: Field,
    title: Field,
    content: Field,
    category: Field,
}

#[derive(Clone)]
pub struct QuerySchema {
    fields: Fields,
    query_parser: QueryParser,
    reader: IndexReader,
}

impl QuerySchema {
    fn tokenizer() -> CangJieTokenizer {
        CangJieTokenizer {
            worker: Arc::new(Jieba::empty()),
            option: TokenizerOption::Unicode,
        }
    }
    fn make_schema() -> Schema {
        let mut schema_builder = Schema::builder();
        let text_indeces = TextFieldIndexing::default()
            .set_tokenizer(CANG_JIE)
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);
        let text_options = TextOptions::default().set_indexing_options(text_indeces);
        schema_builder.add_text_field("title", text_options.clone());
        schema_builder.add_text_field("content", text_options.clone());
        schema_builder.add_text_field("url", TEXT | STORED);
        schema_builder.add_i64_field("date", INDEXED | STORED);
        schema_builder.build()
    }
    pub fn new(path: &str) -> Self {
        let schema = Self::make_schema();
        let index = Index::open_in_dir(path).unwrap();
        index.tokenizers().register(CANG_JIE, Self::tokenizer());
        Self {
            fields: Fields {
                url: schema.get_field("url").unwrap(),
                tags: schema.get_field("tags").unwrap(),
                date: schema.get_field("date").unwrap(),
                title: schema.get_field("title").unwrap(),
                content: schema.get_field("content").unwrap(),
                category: schema.get_field("category").unwrap(),
            },
            query_parser: QueryParser::for_index(&index, vec![title, content]),
            reader: index.reader().unwrap(),
        }
    }
}
