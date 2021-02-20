use std::env;

use actix_web::{get, web, Result};
use chrono::NaiveDate;
use insert::init_schema;
use serde::de::{Deserializer, Error, Unexpected};
use serde::Deserialize;
use tantivy::{collector::TopDocs, schema};

#[path="query/query_schema.rs"] mod query_schema;
#[path="insert/insert.rs"] mod insert;
use tantivy::query::Query;
const path: &str = "./data/";

#[derive(Deserialize)]
struct Thing {
    q: String,
    #[serde(default, deserialize_with = "validate_range")]
    range: Vec<Option<i64>>,
    #[serde(default, deserialize_with = "validate_pages")]
    pages: Vec<u8>,
    #[serde(default, deserialize_with = "validate_terms")]
    terms: Vec<String>,
}
fn validate_q<'de, D>(d: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(String::deserialize(d)?)
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

fn validate_pages<'de, D>(d: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(d)?;
    let mut pages: Vec<u8> = Vec::with_capacity(2);
    let str_values: Vec<&str> = value.split("-").collect();
    let err = Err(Error::invalid_value(
        Unexpected::Str(&value),
        &"invalid format",
    ));
    if str_values.len() != 2 {
        return err;
    }
    for s in &str_values {
        match s.parse::<u8>() {
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
) -> Result<String> {
    let query_schema = qs.get_ref();
    let searcher = query_schema.reader.searcher();
    let q :Box<dyn Query>;
    // match info.query {
    //     Some(ref f) => q = query_schema.query_parser.parse_query(f).unwrap(),
    //     None => q = query_schema.query_parser.parse_query("fkdl").unwrap(),
    // }
    q = query_schema.query_parser.parse_query(&info.q).unwrap();
    println!("{:#?}", q);
    
    let top_docs = searcher.search(&q, &TopDocs::with_limit(10)).unwrap();
    for (_score, doc_addr) in top_docs {
        let retrieved_doc = searcher.doc(doc_addr).unwrap();
        println!("{:#?}", query_schema.schema.to_named_doc(&retrieved_doc));
    }
    // let query = query_schema.query_parser(info.query).unwrap();
    // match info.foo {
    // Some(ref r) => Ok(format!("range {}", r)),
    Ok(format!("range {:?} page: {:?}", info.range, info.pages))
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
    let qs = query_schema::QuerySchema::new(path);
    

    HttpServer::new(move || App::new().data(qs.clone()).service(greet))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
