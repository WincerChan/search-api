use super::init::Blog;
use super::unescape::decode_html_chars;
use std::{borrow::Borrow, io::Read, io::Write, net::TcpStream, str::from_utf8};

use native_tls::TlsConnector;
use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};
use regex::Regex;
use time::{format_description, OffsetDateTime};

fn parse_url(url: &str) -> (String, String) {
    let re = Regex::new(r"https://(.*?)(/.*)").unwrap();
    if !re.is_match(url) {
        panic!("{} not valid https url.", url)
    }
    let cap = re.captures(url).unwrap();
    (cap[1].to_owned(), cap[2].to_owned())
}

fn extract_xml_content(resp: String) -> String {
    let re = Regex::new("(?s).*<feed(.*?)>(.+)</feed>").unwrap();
    if !re.is_match(&resp) {
        panic!("response is not valid. {}", resp)
    }
    let cap = re.captures(&resp).unwrap();
    cap[2].to_owned()
}

fn attrs_mapping(tag: &[u8], e: BytesStart, blog: &mut Blog) {
    match from_utf8(tag).unwrap() {
        "category" => {
            let t = String::from_utf8_lossy(
                e.try_get_attribute("term").unwrap().unwrap().value.as_ref(),
            )
            .to_string();
            if blog.category == "" {
                blog.category = t
            } else {
                blog.tags.push(t)
            }
        }
        "link" => {
            blog.url = String::from_utf8_lossy(
                e.try_get_attribute("href").unwrap().unwrap().value.as_ref(),
            )
            .to_string()
        }
        _ => (),
    };
}

#[test]
fn test_regex() {
    let r = Regex::new(r"([\n|\x20|\xa0])").unwrap();
    let a = r.replace_all("Python 中的 lambda 无法使用赋值（=）符号；", "");
    let b = decode_html_chars(&a);

    println!("after replace, {:?}", b);
}

fn parse_single<'a: 'b, 'b>(mut reader: Reader<&'a [u8]>) -> (Reader<&'b [u8]>, Blog) {
    let mut blog = Blog::default();
    let datetime_format = format_description::parse(
        "[year]-[month]-[day]T[hour]:[minute]:[second][offset_hour]:[offset_minute]",
    )
    .unwrap();
    // let re =
    //     Regex::new(r"(<(div)|(script)[\S|\s]+?/(div)|(script)>)|(</?code>)|(</?strong>)｜(<li>)")
    //         .unwrap();
    let re1 =
        Regex::new(r"(<!.*?>)|(<div[\S|\s]+?/div>)|(<script.*?/script>)|(</?[a-zA-Z]+[\S|\s]*?>)")
            .unwrap();
    let re2 = Regex::new(r"([\n|\x20|\xa0]+)").unwrap();
    loop {
        match reader.read_event().unwrap() {
            Event::CData(e) => {
                let x = from_utf8(e.as_ref()).unwrap();
                let tmp = re1.replace_all(&x, "").to_owned();
                blog.content = decode_html_chars(&re2.replace_all(&tmp, " ").trim());
            }
            Event::Start(e) => match e.name().as_ref() {
                b"title" => {
                    blog.title = decode_html_chars(
                        reader.read_text(e.name()).expect("cannot decode").as_ref(),
                    )
                }
                b"published" => {
                    let date = reader.read_text(e.name()).expect("cannot decode");
                    blog.date = tantivy::DateTime::from_utc(
                        OffsetDateTime::parse(date.borrow(), &datetime_format).unwrap(),
                    );
                }
                _ => (),
            },
            Event::End(e) => match e.name().as_ref() {
                b"entry" => break,
                _ => (),
            },
            // blog tags, category and slug
            Event::Empty(e) => match e.name().as_ref() {
                tag => {
                    attrs_mapping(tag, e.borrow(), &mut blog);
                }
            },
            _ => (),
        }
    }
    (reader, blog)
}

fn parse_xml<'a>(content: String) -> Vec<Blog> {
    let mut reader = Reader::from_str(&content);
    reader.trim_text(true);
    let mut blogs = Vec::new();

    loop {
        let mut _blog = Blog::default();
        match reader.read_event() {
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            // exits the loop when reaching end of file
            Ok(Event::Eof) => break,
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"entry" => {
                    (reader, _blog) = parse_single(reader.to_owned());
                    blogs.push(_blog)
                }
                _ => (),
            },
            _ => (),
        }
    }
    blogs
}

pub fn fetch_atom(url: &str) -> Vec<Blog> {
    let (host, path) = parse_url(url);
    let connector = TlsConnector::new().unwrap();
    let stream = TcpStream::connect(format!("{host}:443")).unwrap();
    let mut stream = connector.connect(&host, stream).unwrap();
    let body = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: Close\r\n\r\n");
    stream.write_all(body.as_bytes()).unwrap();
    let mut res = vec![];
    stream.read_to_end(&mut res).unwrap();
    let xml = extract_xml_content(String::from_utf8_lossy(&res).into_owned());
    parse_xml(xml)
}

#[test]
fn test_fetch_atom() {
    fetch_atom("https://blog.itswincer.com/atom.xml");
}

#[test]
fn test_parse_xml() {
    use std::fs;
    let content = fs::read_to_string("./atom.xml").unwrap();
    parse_xml(content);
}
