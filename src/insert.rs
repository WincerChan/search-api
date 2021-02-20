mod query_schema;

use tantivy::{
    query::QueryParser,
    schema::{
        Field, IndexRecordOption, Schema, TextFieldIndexing, TextOptions, INDEXED, STORED, TEXT,
    },
    DocAddress, Index, IndexReader, LeasedItem, Searcher,
};

struct CreateSchema {
    schema: Schema,
    index_writer: IndexWriter,
}

use cang_jie::{CangJieTokenizer, TokenizerOption, CANG_JIE};
use jieba_rs::Jieba;
use std::{sync::Arc, vec};
fn init_schema() {
    let mut schema_builder = Schema::builder();
    let text_indeces = TextFieldIndexing::default()
        .set_tokenizer(CANG_JIE)
        .set_index_option(IndexRecordOption::WithFreqsAndPositions);
    let text_options = TextOptions::default().set_indexing_options(text_indeces);
    let title = schema_builder.add_text_field("title", text_options.clone());
    let content = schema_builder.add_text_field("content", text_options.clone());
    let date = schema_builder.add_u64_field("date", INDEXED);
    let tags = schema_builder.add_text_field("tags", TEXT);
    let category = schema_builder.add_text_field("category", TEXT);
    let url = schema_builder.add_text_field("url", TEXT);
    let schema = schema_builder.build();
    // let index = Index
}
