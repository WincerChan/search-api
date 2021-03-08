use collector::TopDocs;
use tantivy::{
    collector,
    query::{BooleanQuery, Occur, Query, QueryParser, RangeQuery, TermQuery},
    schema::{Field, IndexRecordOption, Schema, Term, Value},
    Document, Index, IndexReader, SnippetGenerator,
};

use cang_jie::{CangJieTokenizer, TokenizerOption, CANG_JIE};
use jieba_rs::Jieba;
use std::{sync::Arc, vec};

#[derive(Clone)]
pub struct Fields {
    url: Field,
    date: Field,
    tags: Field,
    pub title: Field,
    pub content: Field,
    category: Field,
}

#[derive(Clone)]
pub struct QuerySchema {
    pub fields: Fields,
    pub schema: Schema,
    pub query_parser: QueryParser,
    pub reader: IndexReader,
    worker: Arc<Jieba>
}

impl QuerySchema {
    pub fn tokenizer() -> CangJieTokenizer {
        CangJieTokenizer {
            worker: Arc::new(Jieba::new()),
            option: TokenizerOption::Default{hmm: false},
        }
    }
    pub fn make_terms_query(
        &self,
        terms: &str,
        mut q_vecs: Vec<Box<dyn Query>>,
    ) -> Vec<Box<dyn Query>> {
        for term in terms.split(" ") {
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
        let t = Box::new(TermQuery::new(Term::from_field_text(self.fields.title, word),
        IndexRecordOption::WithFreqsAndPositions));
        let c = Box::new(TermQuery::new(Term::from_field_text(self.fields.content, word),
        IndexRecordOption::WithFreqsAndPositions));
        Box::new(BooleanQuery::new(vec![
            (op, t),
            (op, c)
        ]))
    }

    pub fn make_keyword_query(&self, keyword: &str) -> Box<dyn Query> {
        let (mut must, mut mustnot) = (Vec::new(), Vec::new());
        for key in keyword.split(" ") {
            if key.starts_with("-") {
                mustnot.push(&key[1..])
            } else {
                must.push(key)
            }
        }
        let mut querys: Vec<(Occur, Box<dyn Query>)> = self.worker.cut(&must.join(" "), false).into_iter()
        .filter(|x| x != &" ").map(|x| (Occur::Must, self.make_field_search(x, Occur::Should))).collect();

        let submustnot = self.worker.cut(&mustnot.join(" "), false).into_iter()
        .filter(|x| x != &" ").map(|x| (Occur::Must, self.make_field_search(x, Occur::Should))).collect();

        querys.push((Occur::MustNot, Box::new(BooleanQuery::new(submustnot))));
        Box::new(BooleanQuery::new(querys))
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
        spg.set_max_num_chars(380);
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
        index.tokenizers().register(CANG_JIE, token.clone());
        let title = schema.get_field("title").unwrap();
        let content = schema.get_field("content").unwrap();
        let mut query_parser = QueryParser::for_index(&index, vec![title, content]);
        query_parser.set_conjunction_by_default();
        Self {
            worker: token.worker,
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
