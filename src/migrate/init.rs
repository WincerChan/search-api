use chrono::{DateTime, Utc};
use tantivy::{
    collector::TopDocs,
    query::TermQuery,
    schema::{
        Field, IndexRecordOption, Schema, Term, TextFieldIndexing, TextOptions, INDEXED, STORED,
        STRING
    },
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
    pub tags: Vec<String>,
}

pub fn exist_url(field: Field, url: String, index: Index) -> bool {
    TermQuery::new(Term::from_field_text(field, &url), IndexRecordOption::Basic);
    let reader = index
        .reader_builder()
        .reload_policy(tantivy::ReloadPolicy::OnCommit)
        .try_into()
        .unwrap();
    let searcher = reader.searcher();
    let top_docs = searcher
        .search(
            &TermQuery::new(Term::from_field_text(field, &url), IndexRecordOption::Basic),
            &TopDocs::with_limit(1),
        )
        .expect("search failed");
    return top_docs.len() > 0;
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
    schema_builder.add_text_field("tags", STRING);
    schema_builder.add_text_field("category", STRING);
    schema_builder.add_text_field("url", STRING | STORED);
    return schema_builder.build();
}
pub fn add_doc(schema: Schema, writer: &mut IndexWriter, blog: Blog) {
    let mut doc = Document::new();
    doc.add_text(schema.get_field("title").unwrap(), blog.title);
    doc.add_text(schema.get_field("content").unwrap(), blog.content);
    doc.add_date(schema.get_field("date").unwrap(), &blog.date);
    blog.tags
        .iter()
        .for_each(|tag| doc.add_text(schema.get_field("tags").unwrap(), tag.to_lowercase()));
    doc.add_text(schema.get_field("category").unwrap(), blog.category);
    doc.add_text(schema.get_field("url").unwrap(), blog.url);
    writer.add_document(doc);
}

pub fn build_index(path: &str, schema: Schema) -> Index {
    // check path is exist
    let index = match Path::new(path).join("meta.json").exists() {
        false => Index::create_in_dir(path, schema).unwrap(),
        true => Index::open_in_dir(path).unwrap(),
    };
    index
        .tokenizers()
        .register("UTF-8", QuerySchema::tokenizer());
    index
}

pub fn init_schema(path: &str, source: &str) {
    let schema = build_schema();
    let index = build_index(path, schema.clone());
    let mut index_writer = index.writer(50_000_000).unwrap();

    for blog in blog_to_feed(source) {
        if !exist_url(
            schema.get_field("url").unwrap(),
            blog.url.clone(),
            index.clone(),
        ) {
            add_doc(schema.clone(), &mut index_writer, blog);
        }
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
