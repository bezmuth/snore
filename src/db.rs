use argon2;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use bincode::{Decode, Encode};
use rand::distributions::Alphanumeric;
use rand::*;

#[derive(Encode, Decode, PartialEq, Debug)]
struct UserValue {
    phash: String,
    feeds: Vec<String>,
    token: String,
}

pub fn init() -> sled::Db {
    let db: sled::Db = sled::open("db.sled").unwrap();
    db
}

fn decode_user_value(user_bytes: sled::IVec) -> UserValue {
    bincode::decode_from_slice(&user_bytes[..], bincode::config::standard())
        .unwrap()
        .0
}

pub fn get_users_feeds(user: &str, db: &sled::Db) -> Vec<String> {
    decode_user_value(db.get(user).unwrap().unwrap()).feeds
}

pub fn update_user_feeds(user: &str, db: &sled::Db, feeds: Vec<String>) {
    let mut new_data = decode_user_value(db.get(user).unwrap().unwrap());
    new_data.feeds = feeds;
    let bins = bincode::encode_to_vec(new_data, bincode::config::standard()).unwrap();
    db.remove(user.as_bytes()).unwrap();
    db.insert(user.as_bytes(), bins).unwrap();
    let _ = db.flush();
}

fn update_user_token(user: &str, db: &sled::Db, token: String) {
    let mut new_data = decode_user_value(db.get(user).unwrap().unwrap());
    new_data.token = token;
    let bins = bincode::encode_to_vec(new_data, bincode::config::standard()).unwrap();
    db.remove(user.as_bytes()).unwrap();
    db.insert(user.as_bytes(), bins).unwrap();
    let _ = db.flush();
}

pub fn try_login(username: &str, password: &str, db: &sled::Db) -> Option<String> {
    if let Ok(data) = db.get(username) {
        let decoded = decode_user_value(data.unwrap());
        let parsed_hash = PasswordHash::new(&decoded.phash).unwrap();
        if Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
        {
            let token: String = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(64)
                .map(char::from)
                .collect();
            update_user_token(username, db, token.clone());
            return Some(token);
        }
    }
    println!("Login failed");
    None
}

pub fn try_register(username: &str, password: &str, db: &sled::Db) -> Option<String> {
    println!("{} with {}", username, password);
    if !db.contains_key(username.as_bytes()).unwrap() {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string();
        let new_user = UserValue {
            feeds: vec!["https://100r.co/links/rss.xml".to_string()],
            phash: password_hash,
            token: "1".to_string(),
        };
        let bins = bincode::encode_to_vec(new_user, bincode::config::standard()).unwrap();
        db.insert(username.as_bytes(), bins).unwrap();
        return try_login(username, password, db);
    }
    println!("register failed");
    None
}

pub fn check_token(username: &str, token: &str, db: &sled::Db) -> bool {
    if let Ok(data) = db.get(username) {
        let decoded = decode_user_value(data.unwrap());
        if decoded.token == token {
            return true;
        }
    }
    false
}
