use chrono::NaiveDate;
use serde::{
    de::{Deserializer, Error, Unexpected},
    Deserialize, Serialize,
};
use std::{env, fs, path::Path};
use tantivy::{collector::Count, query::Query, schema::Value};

use async_std::io::{
    prelude::{BufReadExt, WriteExt},
    BufReader,
};
use async_std::os::unix::net::{UnixListener, UnixStream};
use async_std::prelude::*;
mod config;
mod search;
use search::QuerySchema;
mod migrate;

#[derive(Deserialize, Debug)]
struct QueryParams {
    #[serde(default)]
    q: String,
    #[serde(default, deserialize_with = "validate_range")]
    range: Vec<Option<i64>>,
    #[serde(default, deserialize_with = "validate_pages")]
    pages: Vec<usize>,
    #[serde(default, deserialize_with = "validate_terms")]
    terms: Vec<String>,
}

#[derive(Serialize)]
struct Hit {
    url: Value,
    date: Value,
    title: String,
    snippet: String,
}

#[derive(Serialize)]
struct Response {
    count: usize,
    data: Vec<Hit>,
}

fn validate_terms<'de, D>(d: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(d)?;
    let terms = value.split(" ").map(|x| x.to_string());
    Ok(terms.collect())
}

fn validate_range<'de, D>(d: D) -> Result<Vec<Option<i64>>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(d)?;
    let str_values: Vec<&str> = value.split("~").collect();
    let mut ranges: Vec<Option<i64>> = Vec::with_capacity(2);
    let err = Err(Error::invalid_value(
        Unexpected::Str(&value),
        &"ISO 8601 date format",
    ));
    if str_values.len() != 2 {
        return err;
    }
    for strv in str_values {
        if strv.is_empty() {
            ranges.push(None)
        } else {
            match NaiveDate::parse_from_str(strv, "%Y-%m-%d") {
                Ok(v) => ranges.push(Some(v.and_hms(0, 0, 0).timestamp())),
                Err(_) => return err,
            }
        }
    }
    Ok(ranges)
}

fn validate_pages<'de, D>(d: D) -> Result<Vec<usize>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(d)?;
    let mut pages: Vec<usize> = Vec::with_capacity(2);
    let str_values: Vec<&str> = value.split("-").collect();
    let err = Err(Error::invalid_value(
        Unexpected::Str(&value),
        &"invalid format",
    ));
    if str_values.len() != 2 {
        return err;
    }
    for s in &str_values {
        match s.parse::<usize>() {
            Ok(v) => pages.push(v),
            Err(_) => return err,
        }
    }
    Ok(pages)
}

fn execute(info: QueryParams, query_schema: QuerySchema) -> String {
    let mut box_qs: Vec<Box<dyn Query>> = Vec::new();
    let (keyword_query, true_query) = query_schema.make_keyword_query(&info.q);
    if true_query {
        box_qs.push(keyword_query.box_clone())
    }
    box_qs = query_schema.make_terms_query(&info.terms, box_qs);
    box_qs = query_schema.make_date_query(&info.range, box_qs);
    let bool_qs = query_schema.make_bool_query(box_qs);
    let searcher = query_schema.reader.searcher();

    let (top_docs, num) = searcher
        .search(&bool_qs, &(query_schema.make_paginate(&info.pages), Count))
        .unwrap();
    let content_snippet_gen = match true_query {
        true => Some(
            query_schema.make_snippet_gen(keyword_query.box_clone(), query_schema.fields.content),
        ),
        false => None,
    };
    let title_snippet_gen = match true_query {
        true => Some(
            query_schema.make_snippet_gen(keyword_query.box_clone(), query_schema.fields.title),
        ),
        false => None,
    };
    let mut results: Vec<Hit> = Vec::with_capacity(10);
    for (_score, doc_addr) in top_docs {
        let retrieved_doc = searcher.doc(doc_addr).unwrap();
        let values = retrieved_doc.get_sorted_field_values();
        let title = query_schema.make_snippet_value(
            &title_snippet_gen,
            &retrieved_doc,
            values[0].1[0].value(),
        );
        let snippet = query_schema.make_snippet_value(
            &content_snippet_gen,
            &retrieved_doc,
            values[1].1[0].value(),
        );
        results.push(Hit {
            url: values[3].1[0].value().clone(),
            date: values[2].1[0].value().clone(),
            title,
            snippet,
        });
    }
    let se_result = serde_json::json!(Response {
        count: num,
        data: results,
    });
    se_result.to_string()
}

async fn accept_serv(mut stream: UnixStream, qs: QuerySchema) {
    let mut lines = BufReader::new(stream.clone()).lines();
    while let Some(line) = lines.next().await {
        let query_str = line.unwrap();
        let p: QueryParams = serde_json::from_str(query_str.as_str()).unwrap();
        let result = execute(p, qs.clone());
        stream.write(result.as_bytes()).await.unwrap();
        stream.flush().await.unwrap();
    }
    println!("end")
}

async fn loop_accept(socket_path: &str, qs: QuerySchema) {
    if Path::new(socket_path).exists() {
        fs::remove_file(socket_path).unwrap();
    }
    let listener = UnixListener::bind(socket_path).await.unwrap();
    let mut incoming = listener.incoming();
    while let Some(stream) = incoming.next().await {
        let stream = stream.unwrap();
        accept_serv(stream, qs.clone()).await;
    }
}
#[async_std::main]
async fn main() {
    let args = env::args().collect::<Vec<String>>();
    if args.len() == 1 {
        println!(
            "Run with one argument: 
        1. init (Initial tanitvy schema and database. this will empty exists directory.)
        2. migrate (Append new article to exists directory.)
        3. run (run server with unix domain socket.)"
        );
        return;
    }
    let config = config::read_config();
    match args[1].as_ref() {
        "init" => {
            migrate::create_dir(&config.tantivy_db);
            migrate::init_schema(&config.tantivy_db, &config.blog_source);
        }
        "migrate" => {
            migrate::init_schema(&config.tantivy_db, &config.blog_source);
        }
        "run" => {
            let qs = QuerySchema::new(&config.tantivy_db);
            loop_accept(&config.listen_addr, qs).await;
        }
        _ => (),
    }
}
