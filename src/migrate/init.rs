use chrono::{DateTime, Utc};
use tantivy::{
    schema::{IndexRecordOption, Schema, TextFieldIndexing, TextOptions, INDEXED, STORED, TEXT},
    Document, Index, IndexWriter,
};

use super::fetch::blog_to_feed;
use crate::search::QuerySchema;
use std::{fs, path::Path};

pub fn create_dir(path: &str) {
    if Path::new(path).exists() {
        fs::remove_dir_all(path).unwrap();
    }
    fs::create_dir_all(path).unwrap();
}

pub struct Blog {
    pub title: String,
    pub content: String,
    pub url: String,
    pub date: DateTime<Utc>,
    pub category: String,
    pub tags: String,
}

pub fn build_schema() -> Schema {
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
    return schema_builder.build();
}
pub fn add_doc(schema: Schema, writer: &mut IndexWriter, blog: Blog) {
    let mut doc = Document::new();
    doc.add_text(schema.get_field("title").unwrap(), blog.title);
    doc.add_text(schema.get_field("content").unwrap(), blog.content);
    doc.add_date(schema.get_field("date").unwrap(), &blog.date);
    doc.add_text(schema.get_field("tags").unwrap(), blog.tags);
    doc.add_text(schema.get_field("category").unwrap(), blog.category);
    doc.add_text(schema.get_field("url").unwrap(), blog.url);
    writer.add_document(doc);
}

pub fn build_index_writer(path: &str, schema: Schema) -> IndexWriter {
    // check path is exist
    let index = match Path::new(path).join("meta.json").exists() {
        false => Index::create_in_dir(path, schema).unwrap(),
        true => Index::open_in_dir(path).unwrap(),
    };
    index
        .tokenizers()
        .register("UTF-8", QuerySchema::tokenizer());
    return index.writer(50_000_000).unwrap();
}

pub fn init_schema(path: &str, source: &str) {
    let schema = build_schema();
    let mut index_writer = build_index_writer(path, schema.clone());

    for blog in blog_to_feed(source) {
        add_doc(schema.clone(), &mut index_writer, blog);
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
