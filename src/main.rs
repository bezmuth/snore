#[macro_use] extern crate rocket;
use rocket::{State};
use rocket::response::{Flash, Redirect};
use rocket::form::Form;
use rocket_dyn_templates::{Template, context};
use rocket::http::{Cookie, CookieJar};
use chrono::prelude::*;

use std::cmp::Ordering;
use std::collections::{HashMap};
use url::Url;
use std::sync::Mutex;

mod db;
mod feed;

struct Users (sled::Db);
struct FeedCache (Mutex<HashMap<String, Vec<FeedItem>>>);


#[derive(Eq, PartialEq, serde::Serialize, Clone)]
struct FeedItem {
    date :  i64,
    title : String,
    link : String,
    site : String,
    stale_time : i64,
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
fn parse_domain(url: String) -> String{
    let domain = Url::parse(&url);
    match domain {
        Ok(v) => v.host_str().unwrap().to_string(),
        Err(_) => url
    }
}

fn check_cookie(cookies: &CookieJar<'_>, users: &sled::Db, username : &str) -> bool{
    if let Some(token) = cookies.get("token"){
        println!("{}", token.value());
        if db::check_token(username, token.value(), users){
            return true
        }
    }
    false
}

#[get("/login")]
fn login(_users : &State<Users>) -> Template {
    Template::render("login", context! {})
}

#[post("/login", data = "<login_data>")]
fn login_post(cookies: &CookieJar<'_>, users : &State<Users>, login_data : Form<Vec<String>>) -> Flash<Redirect> {
    if let Some(token) = db::tryLogin(&login_data.clone()[0], &login_data[1], &users.0){
        cookies.add(Cookie::new("token", token));
        cookies.add(Cookie::new("user", login_data[0].clone()));
        Flash::success(Redirect::to(format!("/{}", login_data[0])), "Login")
    } else {
        Flash::error(Redirect::to("/login"), "Login Failed")
    }
}


#[post("/<user>/submit", data = "<feeds>")]
fn feedsUpdate(cookies: &CookieJar<'_>, user : &str, users : &State<Users>, feeds : Form<String>) -> Flash<Redirect> {
    if check_cookie(cookies, &users.0, user){
        let _feed_list = db::getUsersFeeds(user, &users.0);
        let feeds = feeds
            .into_inner()
            .lines()
            .map(|st| st.to_string())
            .collect::<Vec<String>>();
        db::update_user_feeds(user, &users.0, feeds);
        return Flash::success(Redirect::to(format!("/{}", user)), "Feeds Updated.");
    }
    Flash::error(Redirect::to("/".to_string()), "Error")
}


#[get("/<user>/settings")]
fn settingsPage(cookies: &CookieJar<'_>, user : &str, users : &State<Users>) -> Template {
    if check_cookie(cookies, &users.0, user){
        let feed_list = db::getUsersFeeds(user, &users.0);
        return Template::render("settings", context! {
            feeds : feed_list,
            username : user,
            logged_in : true,
        });
    }
    Template::render("error", context!{})
}


#[get("/<user>")]
fn userPage(cookies: &CookieJar<'_>, user : &str, users : &State<Users>, feed_cache : &State<FeedCache>) -> Template {
    let _logged_in = false;
    let feed_list = db::getUsersFeeds(user, &users.0);
    let mut feed_items : Vec<FeedItem> = vec![];
    for addr in feed_list{
        if let std::collections::hash_map::Entry::Vacant(e) = feed_cache.0.lock().unwrap().entry(addr.clone()) {
            let mut curr_items = vec![];
            let feed_data: String = ureq::get(&addr)
                .set("Example-Header", "header value")
                .call()
                .unwrap()
                .into_string()
                .unwrap();
            let parsed_feed = feed::parse_feed_data(&feed_data);
            for entry in parsed_feed.entries{
                let item = FeedItem {
                    title : entry.title.unwrap().content,
                    link : entry.links[0].href.clone(),
                    date : entry.published.unwrap_or_else(Utc::now).timestamp(),
                    site : parse_domain(addr.clone()),
                    stale_time : Utc::now().timestamp()
                };
                feed_items.push(item.clone());
                curr_items.push(item.clone());
            }
            e.insert(curr_items);
        } else {
            let items = &mut feed_cache.0.lock().unwrap().get(&addr).unwrap().to_vec();
            // if the last item in the vector (i.e. the most recent) is stale, reload the feed
            if items.last().unwrap().stale_time+600 > Utc::now().timestamp(){
                items.last_mut().unwrap().stale_time = Utc::now().timestamp(); // gotta do this so we dont infinitley reload the feed if nothing new apears
                let feed_data: String = ureq::get(&addr)
                    .set("Example-Header", "header value")
                    .call()
                    .unwrap()
                    .into_string()
                    .unwrap();
                let parsed_feed = feed::parse_feed_data(&feed_data);
                let items_as_links : Vec<String> = items.iter_mut().map(|x| x.link.clone()).collect();
                for entry in parsed_feed.entries{
                    if !(items_as_links.contains(&entry.links[0].href.clone())){
                        items.push(FeedItem {
                            title : entry.title.unwrap().content,
                            link : entry.links[0].href.clone(),
                            date : entry.published.unwrap_or_else(Utc::now).timestamp(),
                            site : parse_domain(addr.clone()),
                            stale_time : Utc::now().timestamp()
                        });
                    }
                }
            }
            feed_items.append(items);
        }

    }
    feed_items.sort();
    feed_items.reverse();

    Template::render("index", context! {
        username: user,
        items: feed_items,
        logged_in: check_cookie(cookies, &users.0, user),
    })
}

#[launch]
pub fn rocket() -> _ {
    let users = Users(db::init());
    let feed_cache = FeedCache(Mutex::new(HashMap::new()));

    rocket::build().mount("/", routes![userPage, settingsPage, feedsUpdate, login, login_post])
                   .attach(Template::fairing())
                   .manage(users)
                   .manage(feed_cache)
}
