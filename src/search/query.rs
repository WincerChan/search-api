use collector::TopDocs;
use tantivy::{
    collector,
    query::{BooleanQuery, Occur, PhraseQuery, Query, QueryParser, RangeQuery, TermQuery},
    schema::{Field, IndexRecordOption, Schema, Term, Value},
    Document, Index, IndexReader, SnippetGenerator,
};

use std::vec;

use crate::tokenizer::{segmentation::cut_string, UTF8Tokenizer};

#[derive(Clone)]
pub struct Fields {
    url: Field,
    date: Field,
    tags: Field,
    pub title: Field,
    pub content: Field,
    category: Field,
}

static DELIMITER: &str = ",";

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
    pub fn make_terms_query(
        &self,
        terms: &str,
        mut q_vecs: Vec<Box<dyn Query>>,
    ) -> Vec<Box<dyn Query>> {
        for term in terms.split(DELIMITER) {
            let p = term.splitn(2, ":").collect::<Vec<&str>>();
            match p[0] {
                "tags" => q_vecs.push(Box::new(TermQuery::new(
                    Term::from_field_text(self.fields.tags, &p[1].to_lowercase()),
                    IndexRecordOption::Basic,
                ))),
                "category" => q_vecs.push(Box::new(TermQuery::new(
                    Term::from_field_text(self.fields.category, &p[1]),
                    IndexRecordOption::Basic,
                ))),
                _ => (),
            }
        }
        return q_vecs;
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

    fn make_subqueries(&self, words: Vec<&str>) -> Vec<(Occur, Box<dyn Query>)> {
        words
            .into_iter()
            .map(|word| {
                (
                    Occur::Must,
                    self.make_field_search(&word.to_lowercase(), Occur::Should),
                )
            })
            .collect()

        // cut_string(&words.join(" "))
        //     .into_iter()
        //     .filter(|x| x != &" ")
        //     .map(|x| {
        //         (
        //             Occur::Must,
        //             self.make_field_search(&x.to_lowercase(), Occur::Should),
        //         )
        //     })
        //     .collect()
    }

    pub fn make_keyword_query(&self, keyword: &str) -> Box<dyn Query> {
        let (mut must, mut mustnot) = (Vec::new(), Vec::new());
        for key in keyword.split(DELIMITER) {
            if key.starts_with("-") {
                mustnot.push(&key[1..])
            } else {
                must.push(key)
            }
        }

        let mut querys = self.make_subqueries(must);
        let mustnot = self.make_subqueries(mustnot);

        querys.push((Occur::MustNot, Box::new(BooleanQuery::new(mustnot))));
        Box::new(BooleanQuery::new(querys))
        // let s = self.query_parser.parse_query(keyword).unwrap();
        // println!("{:#?}", s);
        // s
    }

    pub fn make_date_query(
        &self,
        dates: &str,
        mut q_vecs: Vec<Box<dyn Query>>,
    ) -> Vec<Box<dyn Query>> {
        let dts: Vec<i64> = dates
            .split("~")
            .map(|x| x.parse::<i64>().unwrap())
            .collect();
        let rq: Box<dyn Query> = Box::new(RangeQuery::new_i64(self.fields.date, dts[0]..dts[1]));
        q_vecs.push(rq);
        q_vecs
    }
    pub fn make_snippet_gen(
        &self,
        keyword_query: Box<dyn Query>,
        field: Field,
    ) -> Option<SnippetGenerator> {
        let mut spg =
            SnippetGenerator::create(&self.reader.searcher(), &keyword_query, field).unwrap();
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
            .text()
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

    pub fn make_paginate(&self, pages: &str) -> TopDocs {
        let pgs: Vec<usize> = pages
            .split("-")
            .map(|x| x.parse::<usize>().unwrap())
            .collect();
        let page = pgs[0];
        let size = pgs[1];
        TopDocs::with_limit(size).and_offset((page - 1) * size)
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
        let mut query_parser = QueryParser::for_index(&index, vec![title, content]);
        query_parser.set_conjunction_by_default();
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
            query_parser: query_parser,
            reader: index
                .reader_builder()
                .reload_policy(tantivy::ReloadPolicy::OnCommit)
                .try_into()
                .unwrap(),
        }
    }
}
