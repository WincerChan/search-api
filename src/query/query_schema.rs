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
pub struct Fields {
    url: Field,
    date: Field,
    tags: Field,
    pub title: Field,
    pub content: Field,
    category: Field,
}

#[derive(Clone)]
pub struct QuerySchema {
    pub fields: Fields,
    pub schema: Schema,
    pub query_parser: QueryParser,
    pub reader: IndexReader,
}

impl QuerySchema {
    pub fn tokenizer() -> CangJieTokenizer {
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
    schema_builder.add_i64_field("date", INDEXED);
    schema_builder.add_text_field("tags", TEXT);
    schema_builder.add_text_field("category", TEXT);
    schema_builder.add_text_field("url", TEXT);
        schema_builder.build()
    }
    pub fn new(path: &str) -> Self {
        let index = Index::open_in_dir(path).unwrap();
        let schema = index.schema();
        index.tokenizers().register(CANG_JIE, Self::tokenizer());
        let title = schema.get_field("title").unwrap();
        let content = schema.get_field("content").unwrap();
        Self {
            fields: Fields {
                url: schema.get_field("url").unwrap(),
                tags: schema.get_field("tags").unwrap(),
                date: schema.get_field("date").unwrap(),
                title,
                content,
                category: schema.get_field("category").unwrap(),
            },
            schema,
            query_parser: QueryParser::for_index(&index, vec![title, content]),
            reader: index.reader_builder().reload_policy(tantivy::ReloadPolicy::OnCommit).try_into().unwrap(),
        }
    }
}
