use serde::{
    de::{Deserializer, Error, Unexpected},
    Deserialize, Serialize,
};
use std::{env, fs, path::Path};
use tantivy::{collector::Count, query::Query, schema::Value, SnippetGenerator};

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
    #[serde(deserialize_with = "validate_range")]
    range: Vec<i64>,
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
struct Err {
    err_msg: String,
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

fn validate_range<'de, D>(d: D) -> Result<Vec<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(d)?;
    Ok(value
        .split("~")
        .map(|v| v.parse::<i64>().unwrap())
        .collect())
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
        &"pages format: x-y",
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
    let mut content_gen: Option<SnippetGenerator> = None;
    let mut title_gen: Option<SnippetGenerator> = None;
    let mut box_qs: Vec<Box<dyn Query>> = if &info.q == "" {
        Vec::new()
    } else {
        let kq = query_schema.make_keyword_query(&info.q);
        content_gen = query_schema.make_snippet_gen(kq.box_clone(), query_schema.fields.content);
        title_gen = query_schema.make_snippet_gen(kq.box_clone(), query_schema.fields.title);
        vec![kq]
    };
    box_qs = query_schema.make_terms_query(&info.terms, box_qs);
    box_qs = query_schema.make_date_query(&info.range, box_qs);
    let bool_qs = query_schema.make_bool_query(box_qs);
    let searcher = query_schema.reader.searcher();

    let (top_docs, num) = searcher
        .search(&bool_qs, &(query_schema.make_paginate(&info.pages), Count))
        .unwrap();
    let mut results: Vec<Hit> = Vec::with_capacity(10);
    for (_score, doc_addr) in top_docs {
        let doc = searcher.doc(doc_addr).unwrap();
        let values = doc.get_sorted_field_values();
        let title = query_schema.make_snippet_value(&title_gen, &doc, values[0].1[0].value());
        let snippet = query_schema.make_snippet_value(&content_gen, &doc, values[1].1[0].value());
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
    // println!("accept new client");
    while let Some(line) = lines.next().await {
        let query_str = line.unwrap();
        // println!("{:#?}", query_str);
        let v = serde_json::from_str(query_str.as_str());
        match v {
            Ok(p) => {
                let mut result = execute(p, qs.clone());
                // println!("{:#?}", result);
                result.push('\n');
                match stream.write(result.as_bytes()).await {
                    Ok(_) => (),
                    Err(_) => break,
                }
                stream.flush().await.unwrap();
            }
            Err(e) => {
                let mut ret = serde_json::json!(Err {
                    err_msg: e.to_string()
                })
                .to_string();
                ret.push('\n');
                stream.write(ret.as_bytes()).await.unwrap();
                stream.flush().await.unwrap();
            }
        }
    }
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
