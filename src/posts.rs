use std::{os::linux::raw::stat, thread::sleep, time::Duration, vec};

use mastodon_async::{Registration, helpers::cli, prelude::*};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::conf::{
    read_cw_config, read_disable_mfm, read_disable_sanitize_hashtags,
    read_disable_sanitize_mentions, read_disable_sanitize_mfm, read_instance, read_poster_id,
    read_posting_token, read_visibility,
};

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: Option<String>,
    pub username: Option<String>,
    pub host: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct UserRelation {
    #[serde(rename = "isFollowing")]
    pub is_following: bool,
    #[serde(rename = "isFollowed")]
    pub is_followed: bool,
    #[serde(rename = "hasPendingFollowRequestFromYou")]
    pub has_pending_follow_request_from_you: bool,
}

#[derive(Serialize, Deserialize)]
pub struct UserFollow {
    pub id: String,
    #[serde(rename = "followeeId")]
    pub followee_id: String,
    #[serde(rename = "followerId")]
    pub follower_id: String,
    pub follower: User,
}

#[derive(Serialize, Deserialize)]
pub struct Post {
    pub id: String,
    pub text: Option<String>,
    pub cw: Option<String>,
    pub user: User,
    pub visibility: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Posts {
    posts: Vec<Post>,
}

pub async fn get_posts(mastodon: &Mastodon, user_id: String) -> Vec<Status> {
    let mut statuses = mastodon
        .statuses(&AccountId::new(user_id), Default::default())
        .await
        .unwrap();

    let mut final_statuses: Vec<Status> = vec![];

    for ele in &statuses.initial_items {
        final_statuses.push(ele.clone());
    }

    loop {
        match statuses.next_page().await {
            Ok(statuses) => match statuses {
                Some(statuses) => {
                    for ele in &statuses {
                        final_statuses.push(ele.clone());
                    }
                }
                None => break,
            },
            Err(_) => break,
        }
    }

    final_statuses
}

pub async fn control_follows_for_user(mastodon: &Mastodon, user: &Account) {
    // yolo

    match mastodon.follow(&user.id).await {
        Ok(_) => {
            println!("followed {}", user.acct)
        }
        Err(e) => {
            println!("failed to follow {}: {:?}", user.acct, e)
        }
    }
}

pub async fn get_followers(mastodon: &Mastodon) -> Vec<Account> {
    let mut me = mastodon.follows_me().await.unwrap();

    let mut accounts: Vec<Account> = vec![];

    for ele in &me.initial_items {
        accounts.push(ele.clone());
    }

    loop {
        match me.next_page().await {
            Ok(followers) => match followers {
                Some(followers) => {
                    for ele in followers {
                        accounts.push(ele);
                    }
                }
                None => break,
            },
            Err(_) => break,
        }
    }

    accounts
}

pub fn create_post(text: String) {
    todo!("post note");
}

pub fn remove_mfm(text: String) -> String {
    if !read_disable_mfm() {
        return text;
    }

    let re = Regex::new(r"\$\[\w+ (.*?)\]").unwrap();
    let mut text_process = text;

    for _ in 0..20 {
        text_process = re.replace_all(&text_process, "$1").to_string();
    }

    text_process
}

pub fn sanitize_mentions(text: String) -> String {
    if read_disable_sanitize_mentions() {
        return text;
    }

    let re = Regex::new(r"(@\w+)(@[\w.]+)?").unwrap();
    re.replace_all(&text, "<plain>$1$2</plain>").to_string()
}

pub fn sanitize_hashtags(text: String) -> String {
    if read_disable_sanitize_hashtags() {
        return text;
    }

    let re = Regex::new(r"(#\w+)").unwrap();
    re.replace_all(&text, "<plain>$1</plain>").to_string()
}

pub fn sanitize_formatting(text: String) -> String {
    if read_disable_sanitize_mfm() {
        return text;
    }

    let mut result = String::new();
    let mut chars = text.chars().peekable();

    let mut open_tags = Vec::new();
    let mut open_parentheses = 0;

    let mut processed_closing_tags = Vec::new();

    while let Some(c) = chars.next() {
        match c {
            '$' if chars.peek() == Some(&'[') => {
                chars.next(); // consume '['
                open_tags.push("$[");
                result.push_str("$[");
            }
            '<' if chars.peek() == Some(&'i') => {
                chars.next(); // consume 'i'
                if chars.peek() == Some(&'>') {
                    chars.next(); // consume '>'
                    open_tags.push("<i>");
                    result.push_str("<i>");
                } else {
                    result.push('<');
                    result.push('i');
                }
            }
            '<' if chars.peek() == Some(&'s') => {
                let mut temp = String::new();
                temp.push(c);
                let mut is_small_tag = false;
                while let Some(&next_c) = chars.peek() {
                    chars.next();
                    temp.push(next_c);
                    if temp == "<small>" {
                        open_tags.push("<small>");
                        is_small_tag = true;
                        break;
                    }
                    if next_c == '>' || temp.len() >= 7 {
                        break;
                    }
                }
                if is_small_tag {
                    result.push_str("<small>");
                } else {
                    result.push_str(&temp);
                }
            }
            '*' if chars.peek() == Some(&'*') => {
                chars.next(); // consume second '*'
                if !open_tags.contains(&"**") {
                    open_tags.push("**");
                } else {
                    if let Some(pos) = open_tags.iter().position(|&x| x == "**") {
                        open_tags.remove(pos);
                        processed_closing_tags.push("**");
                    }
                }
                result.push_str("**");
            }
            '(' => {
                open_parentheses += 1;
                result.push(c);
            }
            ')' => {
                if open_parentheses > 0 {
                    open_parentheses -= 1;
                }
                result.push(c);
            }
            ']' => {
                if open_tags.contains(&"$[") {
                    if let Some(pos) = open_tags.iter().position(|&x| x == "$[") {
                        open_tags.remove(pos);
                        processed_closing_tags.push("$[");
                    }
                }
                result.push(']');
            }
            '<' if chars.peek() == Some(&'/') => {
                let mut temp = String::new();
                temp.push(c);
                temp.push(chars.next().unwrap()); // consume '/'

                let mut closing_tag_type = "";
                while let Some(&next_c) = chars.peek() {
                    chars.next();
                    temp.push(next_c);

                    if temp == "</i>" {
                        closing_tag_type = "<i>";
                        break;
                    } else if temp == "</small>" {
                        closing_tag_type = "<small>";
                        break;
                    }

                    if next_c == '>' || temp.len() >= 8 {
                        break;
                    }
                }

                if !closing_tag_type.is_empty() && open_tags.contains(&closing_tag_type) {
                    if let Some(pos) = open_tags.iter().position(|&x| x == closing_tag_type) {
                        open_tags.remove(pos);
                        processed_closing_tags.push(closing_tag_type);
                    }
                }

                result.push_str(&temp);
            }
            _ => result.push(c),
        }
    }

    for tag in open_tags.iter().rev() {
        match *tag {
            "$[" => result.push(']'),
            "<i>" => result.push_str("</i>"),
            "<small>" => result.push_str("</small>"),
            "**" => result.push_str("**"),
            _ => {}
        }
    }

    for _ in 0..open_parentheses {
        result.push(')');
    }

    result
}

#[cfg(test)]
#[serial_test::serial]
mod tests {
    use super::*;

    #[test]
    pub fn test_disable_mfm_basic() {
        let text = "$[x4 Haiiii]".to_string();
        let expected_text = "Haiiii".to_string();

        let sanitized_text = remove_mfm(text);

        assert_eq!(sanitized_text, expected_text);
    }

    #[test]
    pub fn test_disable_mfm_nested() {
        let text = "$[sparkle $[x4 haiii]]".to_string();
        let expected_text = "haiii".to_string();

        let sanitized_text = remove_mfm(text);

        assert_eq!(sanitized_text, expected_text);
    }

    #[test]
    pub fn test_sanitize_mfm() {
        let text = "$[x4 haiii".to_string();
        let expected_text = "$[x4 haiii]".to_string();

        let sanitized_text = sanitize_formatting(text);

        assert_eq!(sanitized_text, expected_text);
    }

    #[test]
    pub fn test_sanitize_mfm_double() {
        let text = "$[sparkle $[x4 haiii".to_string();
        let expected_text = "$[sparkle $[x4 haiii]]".to_string();

        let sanitized_text = sanitize_formatting(text);

        assert_eq!(sanitized_text, expected_text);
    }

    #[test]
    pub fn test_sanitize_mention_local() {
        let text = "@Lunya".to_string();
        let expected_text = "<plain>@Lunya</plain>".to_string();

        let sanitized_text = sanitize_mentions(text);

        assert_eq!(sanitized_text, expected_text);
    }

    #[test]
    pub fn test_sanitize_mention_remote() {
        let text = "@Lunya@fedi.lunya.pet".to_string();
        let expected_text = "<plain>@Lunya@fedi.lunya.pet</plain>".to_string();

        let sanitized_text = sanitize_mentions(text);

        assert_eq!(sanitized_text, expected_text);
    }

    #[test]
    pub fn test_sanitize_hashtags() {
        let text = "#collabmarkov".to_string();
        let expected_text = "<plain>#collabmarkov</plain>".to_string();

        let sanitized_text = sanitize_hashtags(text);

        assert_eq!(sanitized_text, expected_text);
    }
}
