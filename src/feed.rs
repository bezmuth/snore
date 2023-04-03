use bincode::{Decode, Encode};
use chrono::prelude::*;
use feed_rs::parser;
use std::cmp::Ordering;
use std::str;
use std::sync::Arc;
use std::thread::sleep;
use url::Url;

use rocket::tokio::task;
use std::time::Duration; // 1.3.0

#[derive(Eq, PartialEq, serde::Serialize, Clone, Encode, Decode)]
pub struct FeedItem {
    date: i64,
    title: String,
    link: String,
    site: String,
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

pub async fn init() -> Arc<sled::Db> {
    let db: sled::Db = sled::open("feed_db.sled").unwrap();
    let db = Arc::new(db);

    let feed_db = db.clone();

    let forever = task::spawn(async move {
        let duration = Duration::from_secs(600);
        loop {
            sleep(duration);
            if let Ok(Some(first)) = feed_db.first() {
                if let Ok(Some(last)) = feed_db.last() {
                    println!("Running update");
                    for range_item in feed_db.range(first.0..last.0) {
                        let addr = decode_addr(range_item.clone().unwrap().0);
                        let mut feed = decode_feed(range_item.unwrap().1);
                        if let Ok(feed_data) = ureq::get(&addr)
                            .set("Example-Header", "header value")
                            .call()
                        {
                            let parsed_feed = parse_feed_data(&feed_data.into_string().unwrap());
                            let links: Vec<String> =
                                feed.clone().into_iter().map(|x| x.link).collect();
                            for entry in parsed_feed.entries {
                                if !links.contains(&entry.links[0].href.clone()) {
                                    let item = FeedItem {
                                        title: entry.title.unwrap().content,
                                        link: entry.links[0].href.clone(),
                                        date: entry.published.unwrap_or_else(Utc::now).timestamp(),
                                        site: parse_domain(addr.clone()),
                                    };
                                    feed.push(item)
                                }
                            }
                            update_feed(addr, feed, &feed_db).await;
                        }
                    }
                }
            }
        }
    });

    rocket::tokio::spawn(forever);

    return db;
}

fn parse_feed_data(raw_feed: &str) -> feed_rs::model::Feed {
    parser::parse(raw_feed.as_bytes()).unwrap()
}

fn decode_feed(feed_bytes: sled::IVec) -> Vec<FeedItem> {
    bincode::decode_from_slice(&feed_bytes[..], bincode::config::standard())
        .unwrap()
        .0
}

fn decode_addr(addr_bytes: sled::IVec) -> String {
    str::from_utf8(&addr_bytes[..]).unwrap().to_owned()
}

async fn update_feed(addr: String, feed_items: Vec<FeedItem>, feed_db: &sled::Db) {
    feed_db
        .insert(
            addr.as_bytes(),
            bincode::encode_to_vec(feed_items, bincode::config::standard()).unwrap(),
        )
        .unwrap();
    let _ = feed_db.flush_async();
}

pub async fn get_feed(addr: String, feed_db: &sled::Db) -> Vec<FeedItem> {
    if let Ok(Some(db_item)) = feed_db.get(addr.as_bytes()) {
        decode_feed(db_item)
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
            };
            curr_items.push(item.clone());
        }
        update_feed(addr, curr_items.clone(), feed_db).await;
        curr_items
    }
}
