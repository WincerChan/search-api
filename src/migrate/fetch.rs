use super::init::Blog;
use atom_syndication::Feed;
use chrono::Utc;
use regex::Regex;
use ureq;

pub fn blog_to_feed(url: &str) -> Vec<Blog> {
    let body = ureq::get(url).call().unwrap();
    let atom = body.into_string().expect("Save Atom Failed.");
    let feed = atom.parse::<Feed>().expect("Parse Atom Failed.");
    parse_feed(feed)
    // return feed;
}
// fn print_type_of<T>(_: &T) {
//     println!("{}", std::any::type_name::<T>())
// }

pub fn parse_feed(feed: Feed) -> Vec<Blog> {
    let mut blogs = Vec::new();
    let re = Regex::new(r"(<(.*?)>)|(&(.*?);)|[\s]").unwrap();
    for entry in feed.entries() {
        let blog = Blog {
            title: entry.title().to_string(),
            content: re.replace_all(entry.summary().unwrap(), "").to_string(),
            url: entry.links()[0].href().to_string(),
            date: entry.published().unwrap().with_timezone(&Utc),
            category: entry
                .categories()
                .iter()
                .filter(|c| c.label().is_some())
                .next()
                .unwrap()
                .term()
                .to_string(),
            tags: entry
                .categories()
                .iter()
                .filter(|c| c.label().is_none())
                .map(|c| c.term().to_string())
                .collect::<Vec<String>>()
                .join(","),
        };
        blogs.push(blog);
        // println!("{:?}", entry.summary());
        // links.push(link.to_string());
    }
    return blogs;
}
