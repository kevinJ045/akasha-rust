use colored::Colorize;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;

const BASE_URL: &str = "https://akasha.cv/api";

pub fn api_url(name: &str) -> String {
    format!("{}{}", BASE_URL, name)
}

pub fn format_number(num: f64) -> String {
    let formatted = format!("{:.2}", num);
    // Add commas for thousands
    let parts: Vec<&str> = formatted.split('.').collect();
    let mut int_part = parts[0].to_string();
    let mut chars: Vec<char> = int_part.chars().rev().collect();
    for i in (3..chars.len()).step_by(4) {
        chars.insert(i, ',');
    }
    chars.reverse();
    let with_commas: String = chars.into_iter().collect();
    if parts.len() > 1 {
        format!("{}.{}", with_commas, parts[1])
    } else {
        with_commas
    }
}

pub async fn get_user_calculations(user_id: &str) -> Result<Value, Box<dyn Error>> {
    let url = api_url(&format!("/getCalculationsForUser/{}", user_id));
    let response = reqwest::get(&url).await?;
    let data = response.json::<Value>().await?;
    Ok(data["data"].clone())
}

pub async fn get_user_builds(user_id: &str) -> Result<Value, Box<dyn Error>> {
    let url = api_url(&format!(
        "/builds/?sort=critValue&order=-1&size=20&page=1&filter=&uids=&p=&fromId=&li=&uid={}",
        user_id
    ));
    let response = reqwest::get(&url).await?;
    let data = response.json::<Value>().await?;
    Ok(data["data"].clone())
}
