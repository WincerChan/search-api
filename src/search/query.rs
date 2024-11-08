use std::ops::Bound;
use tantivy::{
    collector::{Collector, TopDocs},
    query::{BooleanQuery, Occur, PhraseQuery, Query, QueryParser, RangeQuery, TermQuery},
    schema::{Field, IndexRecordOption, Schema, Term, Value},
    DateTime, DocAddress, Document, Index, IndexReader, SnippetGenerator,
};

use std::vec;

use crate::tokenizer::{segmentation::cut_string, UTF8Tokenizer};

#[derive(Clone)]
pub struct Fields {
    pub url: Field,
    pub date: Field,
    tags: Field,
    pub title: Field,
    pub content: Field,
    pub category: Field,
}

// static DELIMITER: &str = ",";

#[derive(Clone)]
pub struct QuerySchema {
    pub fields: Fields,
    pub schema: Schema,
    pub query_parser: QueryParser,
    pub reader: IndexReader,
}

impl QuerySchema {
    pub fn tokenizer() -> UTF8Tokenizer {
        UTF8Tokenizer {}
    }
    pub fn make_terms_query(&self, terms: Vec<String>, box_qs: &mut Vec<Box<dyn Query>>) {
        let mut q_vecs: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        for term in terms {
            let p = term.splitn(2, ":").collect::<Vec<&str>>();
            let field = match p[0] {
                "tags" => self.fields.tags,
                "category" => self.fields.category,
                _ => continue,
            };
            q_vecs.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(field, &p[1].to_lowercase()),
                    IndexRecordOption::Basic,
                )),
            ))
        }
        if q_vecs.len() != 0 {
            box_qs.push(Box::new(BooleanQuery::new(q_vecs)))
        }
    }

    fn make_field_search(&self, word: &str, op: Occur) -> Box<dyn Query> {
        let chs = cut_string(word);
        let title: Box<dyn Query>;
        let content: Box<dyn Query>;
        if chs.len() == 1 {
            title = Box::new(TermQuery::new(
                Term::from_field_text(self.fields.title, word),
                IndexRecordOption::WithFreqsAndPositions,
            ));
            content = Box::new(TermQuery::new(
                Term::from_field_text(self.fields.content, word),
                IndexRecordOption::WithFreqsAndPositions,
            ));
        } else {
            let mut title_terms: Vec<(usize, Term)> = Vec::with_capacity(chs.len());
            let mut cnt_terms: Vec<(usize, Term)> = Vec::with_capacity(chs.len());
            let mut offset = 0;
            for ch in chs {
                title_terms.push((offset, Term::from_field_text(self.fields.title, ch)));
                cnt_terms.push((offset, Term::from_field_text(self.fields.content, ch)));
                offset += ch.len();
            }
            title = Box::new(PhraseQuery::new_with_offset(title_terms));
            content = Box::new(PhraseQuery::new_with_offset(cnt_terms));
        }
        Box::new(BooleanQuery::new(vec![(op, content), (op, title)]))
    }

    pub fn make_keyword_query(&self, keyword: Vec<String>) -> Result<Vec<Box<dyn Query>>, &str> {
        let (mut must, mut mustnot) = (Vec::new(), Vec::new());
        for key in keyword {
            if key.starts_with("-") {
                let t = key[1..].to_string();
                mustnot.push(t)
            } else if key != "" {
                must.push(key)
            }
        }
        if must.len() == 0 {
            if mustnot.len() != 0 {
                return Err("It is forbidden queries that are only `excluding`.");
            }
            return Ok(vec![]);
        }
        let mut querys: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        let mut must_not: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        for word in must {
            querys.push((
                Occur::Must,
                self.make_field_search(&word.to_lowercase(), Occur::Should),
            ))
        }
        for word in mustnot {
            must_not.push((
                Occur::Must,
                self.make_field_search(&word.to_lowercase(), Occur::Should),
            ))
        }
        if must_not.len() != 0 {
            querys.push((Occur::MustNot, Box::new(BooleanQuery::new(must_not))));
        }
        Ok(vec![Box::new(BooleanQuery::new(querys))])
    }

    fn transform_date_bound(&self, timestamp: i64) -> Bound<DateTime> {
        if timestamp == 0 {
            return Bound::Unbounded;
        }
        let d = DateTime::from_timestamp_secs(timestamp);
        Bound::Included(d)
    }

    pub fn make_date_query(&self, dates: Vec<i64>, box_qs: &mut Vec<Box<dyn Query>>) {
        if dates.len() == 0 {
            return;
        }
        box_qs.push(Box::new(RangeQuery::new_date_bounds(
            "date".to_string(),
            self.transform_date_bound(dates[0]),
            self.transform_date_bound(dates[1]),
        )))
    }
    pub fn make_snippet_gen(
        &self,
        keyword_query: &Box<dyn Query>,
        field: Field,
    ) -> Option<SnippetGenerator> {
        let mut spg =
            SnippetGenerator::create(&self.reader.searcher(), keyword_query, field).unwrap();
        spg.set_max_num_chars(300);
        Some(spg)
    }

    pub fn make_snippet_value(
        &self,
        sp_gen: &Option<SnippetGenerator>,
        doc: &Document,
        field_value: &Value,
    ) -> String {
        let value_str = field_value
            .as_text()
            .unwrap()
            .chars()
            .take(140)
            .skip(0)
            .collect();
        match sp_gen {
            Some(spg) => {
                let sp = spg.snippet_from_doc(doc).to_html();
                if sp == "" {
                    value_str
                } else {
                    sp
                }
            }
            None => value_str,
        }
    }

    pub fn make_paginate(&self, pages: Vec<i64>) -> TopDocs {
        let page = pages[0] as usize;
        let size = pages[1] as usize;
        TopDocs::with_limit(size).and_offset((page - 1) * size)
    }
    pub fn make_paginate_with_sort(
        &self,
        pages: Vec<i64>,
    ) -> impl Collector<Fruit = Vec<(DateTime, DocAddress)>> {
        let page = pages[0] as usize;
        let size = pages[1] as usize;
        TopDocs::with_limit(size)
            .and_offset((page - 1) * size)
            .order_by_fast_field("date", tantivy::Order::Desc)
    }

    pub fn make_bool_query(&self, q_vecs: Vec<Box<dyn Query>>) -> BooleanQuery {
        BooleanQuery::from(
            q_vecs
                .into_iter()
                .map(|q| (Occur::Must, q.box_clone()))
                .collect::<Vec<(Occur, Box<dyn Query>)>>(),
        )
    }
    pub fn new(path: &str) -> Self {
        let index = Index::open_in_dir(path).unwrap();
        let schema = index.schema();
        let token = Self::tokenizer();
        index.tokenizers().register("UTF-8", token.clone());
        let title = schema.get_field("title").unwrap();
        let content = schema.get_field("content").unwrap();
        let query_parser = QueryParser::for_index(&index, vec![title, content]);
        Self {
            fields: Fields {
                url: schema.get_field("url").unwrap(),
                tags: schema.get_field("tags").unwrap(),
                date: schema.get_field("date").unwrap(),
                title,
                content,
                category: schema.get_field("category").unwrap(),
            },
            schema,
            query_parser,
            reader: index
                .reader_builder()
                .reload_policy(tantivy::ReloadPolicy::OnCommit)
                .try_into()
                .unwrap(),
        }
    }
}
