use bincode::{Decode, Encode};
use chrono::prelude::*;
use feed_rs::parser;
use std::cmp::Ordering;
use url::Url;

#[derive(Eq, PartialEq, serde::Serialize, Clone, Encode, Decode)]
pub struct FeedItem {
    date: i64,
    title: String,
    link: String,
    site: String,
    stale_time: i64,
}

impl Ord for FeedItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.date.cmp(&other.date)
    }
}

impl PartialOrd for FeedItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// parses a url and returns the domain
fn parse_domain(url: String) -> String {
    let domain = Url::parse(&url);
    match domain {
        Ok(v) => v.host_str().unwrap().to_string(),
        Err(_) => url,
    }
}

pub fn init() -> sled::Db {
    let db: sled::Db = sled::open("feed_db.sled").unwrap();
    db
}

fn parse_feed_data(raw_feed: &str) -> feed_rs::model::Feed {
    parser::parse(raw_feed.as_bytes()).unwrap()
}

fn decode_feed(feed_bytes: sled::IVec) -> Vec<FeedItem> {
    bincode::decode_from_slice(&feed_bytes[..], bincode::config::standard())
        .unwrap()
        .0
}

fn update_feed(addr: String, feed_items: Vec<FeedItem>, feed_db: &sled::Db) {
    feed_db.remove(addr.as_bytes()).unwrap();
    feed_db
        .insert(
            addr.as_bytes(),
            bincode::encode_to_vec(feed_items, bincode::config::standard()).unwrap(),
        )
        .unwrap();
    let _ = feed_db.flush();
}

pub fn get_feed(addr: String, feed_db: &sled::Db) -> Vec<FeedItem> {
    if let Ok(Some(db_item)) = feed_db.get(addr.as_bytes()) {
        let mut feed = decode_feed(db_item);
        // if the feed goes stale we need to update it
        if feed.last().unwrap().stale_time + 600 < Utc::now().timestamp() {
            feed.last_mut().unwrap().stale_time = Utc::now().timestamp();
            feed_db.remove(addr.clone()).unwrap();
            let feed_data: String = ureq::get(&addr)
                .set("Example-Header", "header value")
                .call()
                .unwrap()
                .into_string()
                .unwrap();
            let parsed_feed = parse_feed_data(&feed_data);
            let links: Vec<String> = feed.clone().into_iter().map(|x| x.link).collect();
            for entry in parsed_feed.entries {
                if !links.contains(&entry.links[0].href.clone()) {
                    let item = FeedItem {
                        title: entry.title.unwrap().content,
                        link: entry.links[0].href.clone(),
                        date: entry.published.unwrap_or_else(Utc::now).timestamp(),
                        site: parse_domain(addr.clone()),
                        stale_time: Utc::now().timestamp(),
                    };
                    feed.push(item)
                }
            }
            update_feed(addr, feed.clone(), feed_db)
        }
        feed
    } else {
        let feed_data: String = ureq::get(&addr)
            .set("Example-Header", "header value")
            .call()
            .unwrap()
            .into_string()
            .unwrap();
        let parsed_feed = parse_feed_data(&feed_data);
        let mut curr_items: Vec<FeedItem> = vec![];
        for entry in parsed_feed.entries {
            let item = FeedItem {
                title: entry.title.unwrap().content,
                link: entry.links[0].href.clone(),
                date: entry.published.unwrap_or_else(Utc::now).timestamp(),
                site: parse_domain(addr.clone()),
                stale_time: Utc::now().timestamp(),
            };
            curr_items.push(item.clone());
        }
        update_feed(addr, curr_items.clone(), feed_db);
        curr_items
    }
}
