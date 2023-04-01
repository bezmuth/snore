use feed_rs::parser;

pub fn parse_feed_data(raw_feed : &str) -> feed_rs::model::Feed{
    parser::parse(raw_feed.as_bytes()).unwrap()
}
