use std::{collections::BTreeMap, ops::Range};

use anyhow::Result;
use maud::{html, Markup};

use crate::app::HTTPReq;

const SURROUNDING_PAGES: u64 = 3;

pub struct PaginationCtl<'a> {
    req: &'a HTTPReq,
    extra_params: BTreeMap<String, String>,
    pages: u64,
    current_page: u64,
    page_size: u8,
}

#[derive(serde::Deserialize)]
struct PageQuery {
    page: u64,
}

impl Default for PageQuery {
    fn default() -> Self {
        Self { page: 1 }
    }
}

pub struct PageSize(u8);

impl<'a> PaginationCtl<'a> {
    pub fn new<S: Into<String> + Clone>(
        req: &'a HTTPReq,
        keep_params: &[S],
        item_count: u64,
    ) -> Result<Self> {
        let keep_params = keep_params.to_vec();
        let keep_params: Vec<String> = keep_params.into_iter().map(|x| x.into()).collect();
        let mut extra_params = BTreeMap::new();
        for (key, value) in req.url().query_pairs() {
            if keep_params.contains(&key.to_string()) {
                extra_params.insert(key.to_string(), value.to_string());
            }
        }
        // clamp page size to a reasonable value
        let page_size = Self::get_pagesize_from_req(req);
        let pages = (item_count + (page_size as u64 - 1)) / page_size as u64;
        let current_page = req.query::<PageQuery>().unwrap_or_default().page;
        Ok(Self {
            req,
            extra_params,
            pages,
            current_page,
            page_size,
        })
    }
    pub fn current_offset(req: &'a HTTPReq, page_size: u32) -> u64 {
        let current_page = req.query::<PageQuery>().unwrap_or_default().page;
        let current_page_size = Self::get_pagesize_from_req(req) as u64;
        (current_page - 1) * current_page_size
    }
    fn get_pagesize_from_req(req: &'a HTTPReq) -> u8 {
        req.ext::<PageSize>()
            .unwrap_or(&PageSize(25))
            .0
            .clamp(5, 100)
    }

    fn left_gap(&self) -> bool {
        self.current_page.saturating_sub(SURROUNDING_PAGES) > 3
    }
    fn left_page_numbers(&self) -> Range<u64> {
        self.current_page.saturating_sub(SURROUNDING_PAGES).max(1)..(self.current_page - 1).max(1)
    }
    fn right_gap(&self) -> bool {
        self.current_page.saturating_add(SURROUNDING_PAGES) < self.pages
    }
    fn right_page_numbers(&self) -> Range<u64> {
        self.current_page + 1..(self.current_page + SURROUNDING_PAGES).min(self.pages)
    }

    pub fn pagination(&self) -> Markup {
        html! {
            @if self.pages > 1 {
                nav.pagination.hide-mobile-t {
                    @if self.current_page != 1 {
                        a.js-prev href="" { "« First" }
                        a href="" { "‹ Prev" }
                    }
                }
            }

            @if self.left_gap() {
                span.page.gap { "…" }
            }

            @for number in self.left_page_numbers() {
                a href="" { (number.to_string()) }
            }

            span.page-current { (self.current_page.to_string()) }

            @for number in self.right_page_numbers() {
                a href="" { (number.to_string()) }
            }

            @if self.right_gap() {
                span.page.gap { "…" }
            }

            @if self.pages > 1 {
                nav.pagination.hide-mobile-t {
                    @if self.current_page != self.pages {
                        a.js-prev href="" { "Next ›" }
                        a href="" { "Last »" }
                    }
                }
            }
        }
    }
}
