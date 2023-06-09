#[macro_use]
extern crate rocket;
use rocket::form::Form;
use rocket::http::{Cookie, CookieJar};
use rocket::response::{Flash, Redirect};
use rocket::State;
use rocket_dyn_templates::{context, Template};
use std::sync::Arc;

mod db;
mod feed;

struct Users(sled::Db);
struct FeedCache(Arc<sled::Db>);

fn check_cookie(cookies: &CookieJar<'_>, users: &sled::Db, username: &str) -> bool {
    if let Some(token) = cookies.get("token") {
        if db::check_token(username, token.value(), users) {
            return true;
        }
    }
    false
}

#[get("/login")]
async fn login(_users: &State<Users>) -> Template {
    Template::render("login", context! {})
}

#[post("/login", data = "<login_data>")]
async fn login_post(
    cookies: &CookieJar<'_>,
    users: &State<Users>,
    login_data: Form<Vec<String>>,
) -> Flash<Redirect> {
    if let Some(token) = db::try_login(&login_data.clone()[0], &login_data[1], &users.0) {
        cookies.add(Cookie::new("token", token));
        cookies.add(Cookie::new("user", login_data[0].clone()));
        Flash::success(Redirect::to(format!("/{}", login_data[0])), "Login")
    } else {
        Flash::error(Redirect::to("/login"), "Login Failed")
    }
}

#[post("/register", data = "<register_data>")]
async fn register_post(
    cookies: &CookieJar<'_>,
    users: &State<Users>,
    register_data: Form<Vec<String>>,
) -> Flash<Redirect> {
    if let Some(token) = db::try_register(&register_data.clone()[0], &register_data[1], &users.0) {
        cookies.add(Cookie::new("token", token));
        cookies.add(Cookie::new("user", register_data[0].clone()));
        Flash::success(Redirect::to(format!("/{}", register_data[0])), "Login")
    } else {
        Flash::error(Redirect::to("/login"), "Register Failed")
    }
}

#[post("/<user>/submit", data = "<feeds>")]
fn feeds_update(
    cookies: &CookieJar<'_>,
    user: &str,
    users: &State<Users>,
    feeds: Form<String>,
) -> Flash<Redirect> {
    if check_cookie(cookies, &users.0, user) {
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

#[post("/logout")]
fn logout(cookies: &CookieJar<'_>) -> Flash<Redirect> {
    cookies.remove(Cookie::named("user"));
    cookies.remove(Cookie::named("token"));
    return Flash::success(Redirect::to("/"), "Logout");
}

#[get("/<user>/settings")]
fn settings_page(cookies: &CookieJar<'_>, user: &str, users: &State<Users>) -> Template {
    if check_cookie(cookies, &users.0, user) {
        let feed_list = db::get_users_feeds(user, &users.0);
        return Template::render(
            "settings",
            context! {
                feeds : feed_list,
                username : user,
                logged_in : true,
            },
        );
    }
    Template::render("error", context! {})
}

#[get("/<user>")]
async fn user_page(
    cookies: &CookieJar<'_>,
    user: &str,
    users: &State<Users>,
    feed_cache: &State<FeedCache>,
) -> Template {
    let feed_list = db::get_users_feeds(user, &users.0);
    let mut feed_items: Vec<feed::FeedItem> = vec![];
    for addr in feed_list {
        let items = feed::get_feed(addr, &feed_cache.0);
        feed_items.append(&mut items.await);
    }
    feed_items.sort();
    feed_items.reverse();
    println!("feed sort");

    Template::render(
        "user",
        context! {
            username: user,
            items: feed_items,
            logged_in: check_cookie(cookies, &users.0, user),
        },
    )
}

#[get("/")]
fn index(cookies: &CookieJar<'_>) -> Template {
    let mut user = "";
    let mut logged_in = false;
    if let Some(username) = cookies.get("user") {
        user = username.value();
        logged_in = true;
    }

    Template::render("index", context! {logged_in: logged_in, username: user})
}
#[launch]
pub async fn rocket() -> _ {
    let users = Users(db::init());
    let feed_cache = FeedCache(feed::init().await);

    rocket::build()
        .mount(
            "/",
            routes![
                index,
                user_page,
                settings_page,
                feeds_update,
                login,
                login_post,
                register_post,
                logout,
            ],
        )
        .attach(Template::fairing())
        .manage(users)
        .manage(feed_cache)
}
