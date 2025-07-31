use std::process::exit;

use clap::Parser;
use configparser::ini::Ini;
use markov::Chain;
use mastodon_async::{
    Data, Language, Mastodon, NewStatus, Registration, helpers::cli, scopes::Scopes,
};

use crate::{
    conf::{read_instance, read_mastodon_config, read_posting_token},
    posts::{control_follows_for_user, get_followers},
};

mod conf;
mod filters;
mod posts;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = false)]
    pub authenticate: bool,

    #[arg(short, long, default_value_t = false)]
    pub write: bool,
}

#[tokio::main]
async fn main() {
    println!("collabmarkov  Copyright (C) 2025  Lunya
This program comes with ABSOLUTELY NO WARRANTY.
This is free software, and you are welcome to redistribute it under certain conditions. Please check LICENSE for more infomation.");

    let args = Args::parse();

    if args.authenticate {
        let registration = Registration::new(format!("https://{}", read_instance()))
            .client_name("collabmarkov2")
            .scopes(Scopes::all())
            .build()
            .await
            .unwrap();

        let mastodon = cli::authenticate(registration).await.unwrap();

        if args.write {
            let data = &mastodon.data;

            let mut config = Ini::new();
            config.load("config.ini").ok();

            config.set("mastodon", "base", Some(data.base.to_string()));
            config.set("mastodon", "client_id", Some(data.client_id.to_string()));
            config.set(
                "mastodon",
                "client_secret",
                Some(data.client_secret.to_string()),
            );
            config.set("mastodon", "token", Some(data.token.to_string()));
            config.set("mastodon", "redirect", Some(data.redirect.to_string()));

            config.write("config.ini").unwrap();
        } else {
            println!("please update the configuration file:");
            println!("{:#?}", mastodon.data);
        }

        exit(0);
    }

    let data = match read_mastodon_config() {
        Some(a) => a,
        None => {
            println!("");
            println!("Please run this app with -a -w or fill out mastodon fields");
            exit(1);
        }
    };
    let mastodon = Mastodon::from(data);

    let followers_ids = get_followers(&mastodon).await;

    for i in &followers_ids {
        control_follows_for_user(&mastodon, i).await;
    }

    // let mut all_posts: Vec<Post> = vec![];

    // for i in 0..followers_ids.len() {
    //     let follower_i = followers_ids[i].clone();
    //     println!(
    //         "processing {} ({}/{})",
    //         follower_i,
    //         i + 1,
    //         followers_ids.len()
    //     );

    //     let mut posts = get_posts(follower_i);
    //     all_posts.append(&mut posts);
    // }

    // let mut total_corpus_length = 0;
    // let mut total_post_count = 0;

    // let mut chain = Chain::new();

    // let filter_config = get_filter_configuration();

    // for post in all_posts {
    //     if post.text.is_none() {
    //         continue;
    //     }

    //     if !check_post(&filter_config, &post) {
    //         continue;
    //     }

    //     total_corpus_length += post.text.clone().unwrap().len();
    //     total_post_count += 1;
    //     chain.feed_str(post.text.unwrap().as_str());
    // }

    // println!("total users: {}", followers_ids.len());
    // println!("total posts: {}", total_post_count);
    // println!("total corpus length: {}", total_corpus_length);

    // let mut str = String::new();

    // for _ in 0..conf::read_multiplier() {
    //     let chunk = chain.generate_str();
    //     str.push_str(&chunk);
    //     str.push_str(" ");
    // }

    // println!("{}", &str);

    // posts::create_post(str);
}
