extern crate utility;

use mongodb::{Client as MongoClient, options::ClientOptions};
use mongodb::bson::doc;

use serde::{Deserialize};

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use std::string::String;
use std::error::Error as stderror;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub token: String,
    mongodb_connection_string: String
}

pub async fn get_mongodb_tools() -> MongoClient {
    let mongodb_connection_string = read_user_from_file("config.json").unwrap().mongodb_connection_string;
    let mut mongodb_client_options = ClientOptions::parse(mongodb_connection_string).await.unwrap();
    mongodb_client_options.app_name = Some("doxa-bot".to_string());
    let mongodb_client = MongoClient::with_options(mongodb_client_options).unwrap();
    return mongodb_client;
}

pub fn read_user_from_file<P: AsRef<Path>>(path: P) -> Result<Config, Box<dyn stderror>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let c = serde_json::from_reader(reader)?;

    Ok(c)
}