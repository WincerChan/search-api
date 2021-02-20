use actix_web::{get, web, Result};
use chrono::NaiveDate;
use serde::de::{Deserializer, Error, Unexpected};
use serde::Deserialize;
mod query_schema;

const path: &str = "./data/";

#[derive(Deserialize)]
struct Thing {
    #[serde(default, deserialize_with = "validate_range")]
    range: Vec<Option<i64>>,
    #[serde(default, deserialize_with = "validate_pages")]
    pages: Vec<u8>,
    #[serde(default, deserialize_with = "validate_terms")]
    terms: Vec<String>,
    query: Option<String>,
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
    // match info.foo {
    // Some(ref r) => Ok(format!("range {}", r)),
    Ok(format!("range {:?} page: {:?}", info.range, info.pages))
    // None => Ok("fdjklf".to_lowercase()),
    // }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use actix_web::{App, HttpServer};
    let qs = query_schema::QuerySchema::new(path);

    HttpServer::new(move || App::new().data(qs.clone()).service(greet))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
