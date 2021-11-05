use config::read::Network;
use ipc::encode_result;
use serde::Serialize;
use std::{
    env,
    fmt::Debug,
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    path::Path,
};
use tantivy::collector::Count;

use std::os::unix::net::UnixListener;
use std::thread;
mod config;
mod search;
use search::QuerySchema;
mod ipc;
mod migrate;
mod tokenizer;

static DEFAULT_MAX_SIZE: usize = 8;

#[derive(Serialize)]
struct Hit {
    url: String,
    date: String,
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

fn execute(
    pages: Vec<i64>,
    range: Vec<String>,
    terms: Vec<String>,
    q: Vec<String>,
    query_schema: &QuerySchema,
) -> String {
    let kq = query_schema.make_keyword_query(q);
    if kq.is_err() {
        return format!("{{\"err_msg\": \"{}\"}}\n", kq.unwrap_err().to_string());
    }
    let mut box_qs = kq.unwrap();
    query_schema.make_terms_query(terms, &mut box_qs);
    query_schema.make_date_query(range, &mut box_qs);
    if box_qs.len() == 0 {
        return format!("{{\"err_msg\": \"It is forbidden queries that are empty.\"}}\n");
    }
    let content_gen = query_schema.make_snippet_gen(&box_qs[0], query_schema.fields.content);
    let title_gen = query_schema.make_snippet_gen(&box_qs[0], query_schema.fields.title);

    let bool_qs = query_schema.make_bool_query(box_qs);
    let searcher = query_schema.reader.searcher();

    let (top_docs, num) = searcher
        .search(&bool_qs, &(query_schema.make_paginate(pages), Count))
        .expect("Search Failed");
    let mut results: Vec<Hit> = Vec::with_capacity(DEFAULT_MAX_SIZE);
    for (_score, doc_addr) in top_docs {
        let doc = searcher.doc(doc_addr).expect("Not Found Document Address");
        let values = doc.get_sorted_field_values();
        let title = query_schema.make_snippet_value(&title_gen, &doc, values[0].1[0].value());
        let snippet = query_schema.make_snippet_value(&content_gen, &doc, values[1].1[0].value());
        results.push(Hit {
            url: values[3].1[0].value().text().expect("Err Url").to_string(),
            date: values[2].1[0]
                .value()
                .date_value()
                .expect("Err date")
                .to_rfc3339(),
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

fn handle_client<T: Write + Read + Debug>(stream: &mut T, qs: QuerySchema) {
    println!("new client: {:?}", stream);
    loop {
        let params: (Vec<i64>, Vec<String>, Vec<String>, Vec<String>);
        match ipc::extract_params(stream) {
            Ok(p) => params = p,
            Err(_) => break,
        }
        let (p, r, t, q) = params;
        let result = execute(p, r, t, q, &qs);
        let result = encode_result(result);
        match stream.write_all(&result) {
            Ok(_) => (),
            _ => break,
        }
    }
    println!("closed connection. {:?}", stream);
}

fn loop_handle<S, L, E>(listener: L, qs: QuerySchema)
where
    S: Write + Read + Sync + Send + 'static + Debug,
    L: IntoIterator<Item = Result<S, E>>,
    E: Debug,
{
    for stream in listener.into_iter() {
        let tmp = qs.clone();
        match stream {
            Ok(mut stream) => {
                thread::spawn(move || handle_client(&mut stream, tmp));
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
}

fn dev_accept(socket: &Network, qs: QuerySchema) {
    let tcp = TcpListener::bind(&socket.listen_addr).expect("Bind to port error");
    for stream in tcp.incoming().into_iter() {
        match stream {
            Ok(mut stream) => {
                stream
                    .write_all("Arguments: Page, Range, Tags, Keywords\r\n> ".as_bytes())
                    .expect("Failed connect");
                loop {
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    let mut resp = String::new();
                    reader.read_line(&mut resp).expect("Failed to read line.");
                    let raw = resp.strip_suffix("\r\n").expect("failed ");
                    let args: Vec<&str> = raw.split(",").collect();
                    let mut result = "Invalid Arguments. ".to_owned();
                    if args.len() == 4 {
                        result = execute(
                            args[0]
                                .split("-")
                                .map(|s| s.parse().unwrap())
                                .collect::<Vec<_>>(),
                            args[1]
                                .split("~")
                                .map(|s| s.to_string())
                                .collect::<Vec<_>>(),
                            args[2]
                                .split(" ")
                                .map(|s| s.to_string())
                                .collect::<Vec<_>>(),
                            args[3]
                                .split(" ")
                                .map(|s| s.to_string())
                                .collect::<Vec<_>>(),
                            &qs,
                        );
                    }
                    stream
                        .write_all((result + "\r\n> ").as_bytes())
                        .expect("Failed send");
                }
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
}

fn socket_accept(socket: &Network, qs: QuerySchema) {
    if Path::new(&socket.listen_addr).exists() {
        fs::remove_file(&socket.listen_addr).unwrap();
    }
    println!("Listening on file {:}", socket.listen_addr);
    match &socket.listen_type[0..] {
        "uds" => {
            let uds = UnixListener::bind(&socket.listen_addr).expect("Binding to file error");
            loop_handle(&mut uds.incoming(), qs)
        }
        "tcp" => {
            let tcp = TcpListener::bind(&socket.listen_addr).expect("Binding to port error");
            loop_handle(&mut tcp.incoming(), qs)
        }
        _ => (),
    }
}

fn main() {
    let args = env::args().collect::<Vec<String>>();
    if args.len() == 1 {
        println!(
            "Run with one argument: 
        1. init (Initial tanitvy schema and database. this will empty exists directory.)
        2. migrate (Append new article to exists directory.)
        3. run (run server with unix domain socket.)
        4. dev (run server with tcp and accept raw args.)"
        );
        return;
    }
    let config = config::read_config();
    match args[1].as_ref() {
        "init" => {
            migrate::create_dir(&config.database.tantivy_db);
            migrate::init_schema(&config.database.tantivy_db, &config.database.blog_source);
        }
        "migrate" => {
            migrate::init_schema(&config.database.tantivy_db, &config.database.blog_source);
        }
        "run" => {
            let qs = QuerySchema::new(&config.database.tantivy_db);
            socket_accept(&config.network, qs);
        }
        "dev" => {
            let qs = QuerySchema::new(&config.database.tantivy_db);
            dev_accept(&config.network, qs);
        }
        _ => (),
    }
}
