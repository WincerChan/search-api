use config::read::Network;
use ipc::encode_result;
use migrate::init_schema;
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
use time::{format_description, Date};

use std::os::unix::net::UnixListener;
use std::{process::exit, thread};
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
    category: String,
    tags: Vec<String>,
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
    range: Vec<i64>,
    terms: Vec<String>,
    q: Vec<String>,
    query_schema: &QuerySchema,
) -> String {
    let kq = query_schema.make_keyword_query(q.clone());
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

    let mut results: Vec<Hit> = Vec::with_capacity(DEFAULT_MAX_SIZE);
    let (results, num) = if !q.is_empty() {
        let (top_docs, num) = searcher
            .search(&bool_qs, &(query_schema.make_paginate(pages), Count))
            .expect("Search Failed");
        for (_score, doc_addr) in top_docs {
            let doc = searcher.doc(doc_addr).expect("Not Found Document Address");
            let values = doc.get_sorted_field_values();
            let title = query_schema.make_snippet_value(&title_gen, &doc, values[0].1[0]);
            let snippet = query_schema.make_snippet_value(&content_gen, &doc, values[1].1[0]);
            results.push(Hit {
                url: values[5].1[0].as_text().expect("Err Url").to_string(),
                date: values[2].1[0]
                    .as_date()
                    .expect("Err date")
                    .into_utc()
                    .to_string(),
                category: values[4].1[0].as_text().expect("Err Category").to_string(),
                tags: values[3]
                    .1
                    .to_vec()
                    .into_iter()
                    .map(|x| x.as_text().expect("Err tag").to_string())
                    .collect(),
                title,
                snippet,
            });
        }
        (results, num)
    } else {
        let (top_docs, num) = searcher
            .search(
                &bool_qs,
                &(query_schema.make_paginate_with_sort(pages), Count),
            )
            .expect("Search Failed");
        for (_score, doc_addr) in top_docs {
            let doc = searcher.doc(doc_addr).expect("Not Found Document Address");
            let values = doc.get_sorted_field_values();
            let title = query_schema.make_snippet_value(&title_gen, &doc, values[0].1[0]);
            let snippet = query_schema.make_snippet_value(&content_gen, &doc, values[1].1[0]);
            results.push(Hit {
                url: values[5].1[0].as_text().expect("Err Url").to_string(),
                date: values[2].1[0]
                    .as_date()
                    .expect("Err date")
                    .into_utc()
                    .to_string(),
                category: values[4].1[0].as_text().expect("Err Category").to_string(),
                tags: values[3]
                    .1
                    .to_vec()
                    .into_iter()
                    .map(|x| x.as_text().expect("Err tag").to_string())
                    .collect(),
                title,
                snippet,
            });
        }
        (results, num)
    };
    let se_result = serde_json::json!(Response {
        count: num,
        data: results,
    });
    se_result.to_string()
}

fn handle_client<T: Write + Read + Debug>(stream: &mut T, qs: QuerySchema) {
    println!("new client: {:?}", stream);
    loop {
        let params: (Vec<i64>, Vec<i64>, Vec<String>, Vec<String>);
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

fn transform_date(date_str: &str) -> i64 {
    if date_str == "" {
        return 0;
    }
    let date_format = format_description::parse("[year]-[month]-[day]").unwrap();

    match Date::parse(date_str, &date_format) {
        Ok(date) => date.midnight().assume_utc().unix_timestamp(),
        Err(e) => {
            println!("Error parsing date `{}`, {}", date_str, e);
            0
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
                                .map(|d| transform_date(d))
                                .collect::<Vec<_>>(),
                            args[2]
                                .split(" ")
                                .map(|s| s.to_string())
                                .collect::<Vec<_>>(),
                            args[3]
                                .split(" ")
                                .map(|s| s.to_string())
                                .filter(|s| !s.is_empty())
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

fn print_usage(program: String) {
    println!(
        "Usage: {} [run|dev]
    1. run (run server with unix domain socket.)
    2. dev (run server with tcp and accept raw args.)",
        program
    );
    exit(0)
}

fn run(config_path: String, instruction: &str) {
    let config = config::read_config(config_path);
    migrate::create_dir(&config.database.tantivy_db);
    init_schema(&config.database.tantivy_db, &config.database.atom_url);
    migrate::scheduled_load_schema(
        &config.database.tantivy_db,
        config.database.atom_url,
        config.database.update_interval,
    );
    match instruction {
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

fn locate_config_file() -> String {
    let paths = ["./", "/etc/", "/usr/local/etc/"];
    for path in paths {
        match Path::new(path).join("search.toml").exists() {
            true => {
                return Path::new(path)
                    .join("search.toml")
                    .to_str()
                    .unwrap()
                    .to_owned()
            }
            false => (),
        }
    }
    println!(
        "Cannot find config file. Put `search.toml` file in below paths:
    1. currenct directory
    2. /etc/
    3. /usr/local/etc/"
    );
    exit(0)
}

fn main() {
    let args = env::args().collect::<Vec<String>>();
    if args.len() == 1 {
        print_usage(args[0].clone());
        return;
    }
    let conf_path = locate_config_file();
    run(conf_path, &args[1]);
}
