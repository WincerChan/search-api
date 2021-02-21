use actix_web::{get, web, HttpResponse, Result};
use chrono::NaiveDate;
use insert::init_schema;
use serde::de::{Deserializer, Error, Unexpected};
use serde::{Deserialize, Serialize};
use std::{env, time::Instant};
use tantivy::{
    collector::{Count, TopDocs},
    query::QueryClone,
    schema::Value,
    SnippetGenerator,
};

#[path = "insert/insert.rs"]
mod insert;
#[path = "query/query_schema.rs"]
mod query_schema;
use tantivy::query::Query;
const PATH: &str = "/tmp/data/";

#[derive(Deserialize)]
struct Thing {
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

#[get("/")]
async fn greet(
    info: web::Query<Thing>,
    qs: web::Data<query_schema::QuerySchema>,
) -> Result<HttpResponse> {
    let query_schema = qs.get_ref();
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
    // let query = query_schema.query_parser(info.query).unwrap();
    // match info.foo {
    // Some(ref r) => Ok(format!("range {}", r)),
    Ok(HttpResponse::Ok().json(Response {
        count: num,
        data: results,
    }))
    // None => Ok("fdjklf".to_lowercase()),
    // }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let v = env::args().collect::<Vec<String>>();
    if v.len() >= 2 {
        init_schema(&v[1]);
        return Ok(());
    }
    use actix_web::{App, HttpServer};
    let qs = query_schema::QuerySchema::new(PATH);

    HttpServer::new(move || App::new().data(qs.clone()).service(greet))
        .bind("127.0.0.1:7007")?
        .run()
        .await
}
