use std::iter::FromIterator;

#[derive(PartialEq, Eq)]
enum STATE {
    Normal,
    Escape,
}

static HTML_CHARS: &'static [(&'static str, char)] = &[
    ("#34", '\u{0022}'),
    ("#39", '\u{0027}'),
    ("#43", '\u{002b}'),
    ("amp", '\u{0026}'),
    ("gt", '\u{003E}'),
    ("hellip", '\u{2026}'),
    ("ldquo", '\u{201C}'),
    ("lsquo", '\u{2018}'),
    ("lt", '\u{003C}'),
    ("ndash", '\u{2013}'),
    ("quot", '\u{0022}'),
    ("rdquo", '\u{201D}'),
    ("rsquo", '\u{2019}'),
];

#[test]
fn test_f() {
    assert_eq!(' ', '\u{a0}');
    let test_str = "你好 礼服";
    for x in test_str.chars() {
        println!("{:?}", x);
    }
}

#[test]
fn test_binary_search() {
    let mut v: Vec<char> = vec![];
    v.push('\u{0}');
    let s = String::from_iter(v);
    println!("{}", s.is_empty());
    match HTML_CHARS.binary_search_by(|&(name, _)| name.cmp("lsquo")) {
        Err(x) => println!("err match: {}", x),
        Ok(idx) => {
            let (_, u) = HTML_CHARS[idx];
            println!("{:?}", u)
        }
    }
}

fn do_match(v: &Vec<char>) -> Option<char> {
    let x = String::from_iter(v);
    match HTML_CHARS.binary_search_by(|&(name, _)| name.cmp(&x)) {
        Err(_) => None,
        Ok(idx) => {
            let (_, u) = HTML_CHARS[idx];
            Some(u)
        }
    }
}

fn do_unescape(raw: &str, writer: &mut Vec<char>) {
    let mut buf: Vec<char> = Vec::with_capacity(8);
    let mut state = STATE::Normal;
    for char in raw.chars() {
        match state {
            STATE::Normal if char == '&' => {
                state = STATE::Escape;
                buf.clear();
            }
            STATE::Escape if char == ';' => {
                match do_match(&buf) {
                    Some(c) => writer.push(c),
                    _ => (),
                };
                buf.clear();
                state = STATE::Normal;
            }
            STATE::Escape if buf.len() == 7 => {
                writer.append(&mut buf);
                writer.push(char);
                buf.clear();
                state = STATE::Normal
            }
            STATE::Escape if char.len_utf8() == 1 => buf.push(char),
            STATE::Escape => {
                state = STATE::Normal;
                writer.push(char)
            }
            STATE::Normal => writer.push(char),
        }
    }
}

pub fn decode_html_chars(html: &str) -> String {
    // let mut reader = Cursor::new(html.as_bytes());
    let mut writer: Vec<char> = Vec::with_capacity(html.chars().count());
    do_unescape(html, &mut writer);
    String::from_iter(writer)
    // from_utf8(writer.get_ref()).unwrap().to_owned()
}

#[test]
fn test_decode_htlm() {
    let after = decode_html_chars("&lt;你好，&你姐姐&gt;");
    println!("{:?}", after);
}
