A super simple "blazingly fast" rust clone of [Jes Olson's](https://j3s.sh/) [vore.website](https://vore.website/), written in an afternoon to learn
a bit more about rust and web development.

Stuff to note:
* Uses an embedded database (not sql) like vore.
* All feeds are accesible by all users.
* The async rss update might cause some issues
* Written by an insane person that has no idea how to do web security properly
* I kinda wanna implement activitypub following at some point (i.e. follow mastodon users and have it show up in the feed)

Run with `cargo run`

