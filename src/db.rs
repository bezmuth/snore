use rocket::log::private::Metadata;
use sled;
use bincode::{config, Decode, Encode};
use rand::{distributions::Alphanumeric, Rng};

#[derive(Encode, Decode, PartialEq, Debug)]
struct userValue {
    phash : String,
    feeds : Vec<String>,
    token : String
}


pub fn init() -> sled::Db {
    let exampleData = userValue {
        feeds : vec!["https://xeiaso.net/blog.rss".to_string(), "https://100r.co/links/rss.xml".to_string(), "https://asahilinux.org/blog/index.xml".to_string()],
        phash : "deadbeef".to_string(),
        token : "1".to_string(),
    };
    let bins = bincode::encode_to_vec(exampleData, bincode::config::standard()).unwrap();
    let db: sled::Db = sled::open("db.sled").unwrap();
    db.insert(b"reocha", bins);
    return db
}

fn decodeUserValue(userBytes: sled::IVec) -> userValue {
    bincode::decode_from_slice(&userBytes[..], bincode::config::standard()).unwrap().0
}

pub fn getUsersFeeds(user : &str, db : &sled::Db) -> Vec<String> {
    decodeUserValue(db.get(user).unwrap().unwrap()).feeds
}

pub fn update_user_feeds(user : &str, db : &sled::Db, feeds : Vec<String>) {
    let mut newData = decodeUserValue(db.get(user).unwrap().unwrap());
    newData.feeds = feeds;
    let bins = bincode::encode_to_vec(newData, bincode::config::standard()).unwrap();
    db.remove(user.as_bytes());
    db.insert(user.as_bytes(), bins);
}

fn update_user_token(user : &str, db : &sled::Db, token : String){
    let mut newData = decodeUserValue(db.get(user).unwrap().unwrap());
    newData.token = token;
    let bins = bincode::encode_to_vec(newData, bincode::config::standard()).unwrap();
    db.remove(user.as_bytes());
    db.insert(user.as_bytes(), bins);
}

pub fn tryLogin(username : &str, password : &str, db : &sled::Db) -> Option<String> {
    if let Ok(data) = db.get(username) {
        let decoded = decodeUserValue(data.unwrap());
        if decoded.phash == password{
            let token : String = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(64)
                .map(char::from)
                .collect();
            update_user_token(username, db, token.clone());
            return Some(token)
        }
    }
    return None
}

pub fn check_token(username : &str, token: &str, db : &sled::Db) -> bool{
    if let Ok(data) = db.get(username) {
        let decoded = decodeUserValue(data.unwrap());
        if decoded.token == token{
            return true
        }
    }
    return false
}
