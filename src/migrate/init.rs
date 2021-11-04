use tantivy::{
    schema::{IndexRecordOption, Schema, TextFieldIndexing, TextOptions, INDEXED, STORED, TEXT},
    Index,
};

use crate::search::QuerySchema;
use serde_json::Value;
use std::{fs, path::Path};

pub fn create_dir(path: &str) {
    if Path::new(path).exists() {
        fs::remove_dir_all(path).unwrap();
    }
    fs::create_dir_all(path).unwrap();
}

pub fn init_schema(path: &str, source: &str) {
    let mut schema_builder = Schema::builder();
    let text_indeces = TextFieldIndexing::default()
        .set_tokenizer("UTF-8")
        .set_index_option(IndexRecordOption::WithFreqsAndPositions);
    let text_options = TextOptions::default().set_indexing_options(text_indeces);
    schema_builder.add_text_field("title", text_options.clone() | STORED);
    schema_builder.add_text_field("content", text_options.clone() | STORED);
    // schema_builder.add_i64_field("date", INDEXED | STORED);
    // make date file type to date
    schema_builder.add_date_field("date", INDEXED | STORED);
    schema_builder.add_text_field("tags", TEXT);
    schema_builder.add_text_field("category", TEXT);
    schema_builder.add_text_field("url", TEXT | STORED);
    let schema = schema_builder.build();
    let index = Index::create_in_dir(path, schema.clone()).unwrap();
    index
        .tokenizers()
        .register("UTF-8", QuerySchema::tokenizer());

    let contents = fs::read_to_string(source).expect("Can't Open the file.");
    let v: Value = serde_json::from_str(&contents).unwrap();
    let mut index_writer = index.writer(50_000_000).unwrap();
    for x in v.as_array().unwrap() {
        let d = schema.parse_document(&x.to_string()).unwrap();
        index_writer.add_document(d);
    }
    index_writer.commit().unwrap();
    // let reader = index
    //     .reader_builder()
    //     .reload_policy(ReloadPolicy::OnCommit)
    //     .try_into()
    //     .unwrap();

    // let searcher = reader.searcher();
    // let q_p = QueryParser::for_index(&index, vec![title, content]);
    // let q = q_p.parse_query("Hello").unwrap();
    // let top = searcher.search(&q, &TopDocs::with_limit(10)).unwrap();
    // for (_score, doc_address) in top {
    //     let retrieved_doc = searcher.doc(doc_address).unwrap();
    //     println!(" a ?{:#?}", s2.to_named_doc(&retrieved_doc));
    // }
}
