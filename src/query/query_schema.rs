use actix_web::rt::System;
use collector::TopDocs;
use tantivy::{
    collector,
    query::{BooleanQuery, Occur, Query, QueryParser, RangeQuery, TermQuery},
    schema::{
        Field, IndexRecordOption, Schema, Term, TextFieldIndexing, TextOptions, Value, INDEXED,
        STORED, TEXT,
    },
    DocAddress, Document, Index, IndexReader, LeasedItem, Searcher, SnippetGenerator,
};

use cang_jie::{CangJieTokenizer, TokenizerOption, CANG_JIE};
use jieba_rs::Jieba;
use std::{cmp::min, ops::Range, sync::Arc, time::SystemTime, vec};

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
}

impl QuerySchema {
    pub fn tokenizer() -> CangJieTokenizer {
        CangJieTokenizer {
            worker: Arc::new(Jieba::empty()),
            option: TokenizerOption::Unicode,
        }
    }
    pub fn make_terms_query(
        &self,
        terms: &Vec<String>,
        mut q_vecs: Vec<Box<dyn Query>>,
    ) -> Vec<Box<dyn Query>> {
        for term in terms {
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

    pub fn make_keyword_query(&self, keyword: &str) -> (Box<dyn Query>, bool) {
        (
            self.query_parser.parse_query(keyword).unwrap(),
            keyword != "",
        )
    }

    pub fn make_date_query(
        &self,
        dates: &Vec<Option<i64>>,
        mut q_vecs: Vec<Box<dyn Query>>,
    ) -> Vec<Box<dyn Query>> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let r = match dates[..] {
            [Some(start), Some(stop)] => start..stop,
            [Some(start), None] => start..(now as i64),
            [None, Some(stop)] => 0..stop,
            _ => return q_vecs,
        };
        let rq: Box<dyn Query> = Box::new(RangeQuery::new_i64(self.fields.date, r));
        q_vecs.push(rq);
        q_vecs
    }
    pub fn make_snippet_gen(
        &self,
        keyword_query: Box<dyn Query>,
        field: Field,
    ) -> SnippetGenerator {
        let mut sp =
            SnippetGenerator::create(&self.reader.searcher(), &keyword_query, field).unwrap();
        sp.set_max_num_chars(380);
        sp
    }

    pub fn make_snippet_value(
        &self,
        sp_gen: &Option<SnippetGenerator>,
        doc: &Document,
        field_value: &Value,
    ) -> String {
        match sp_gen {
            Some(spg) => spg.snippet_from_doc(doc).to_html(),
            None => {
                let t = field_value.text().unwrap();
                t.chars().take(140).skip(0).collect()
            }
        }
    }

    pub fn make_paginate(&self, pages: &Vec<usize>) -> TopDocs {
        if pages.len() == 0 {
            TopDocs::with_limit(1)
        } else {
            let page = pages[0];
            let size = pages[1];
            TopDocs::with_limit(size).and_offset((page - 1) * size)
        }
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
        index.tokenizers().register(CANG_JIE, Self::tokenizer());
        let title = schema.get_field("title").unwrap();
        let content = schema.get_field("content").unwrap();
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
            query_parser: QueryParser::for_index(&index, vec![title, content]),
            reader: index
                .reader_builder()
                .reload_policy(tantivy::ReloadPolicy::OnCommit)
                .try_into()
                .unwrap(),
        }
    }
}
