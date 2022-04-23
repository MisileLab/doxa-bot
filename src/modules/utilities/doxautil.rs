extern crate utility;

use mongodb::{
    Client as MongoClient, 
    options::ClientOptions,
    bson::doc
};

use serde::Deserialize;

use std::{
    fs::File,
    io::BufReader,
    path::Path,
    string::String,
    error::Error as stderror
};

#[derive(Deserialize, Debug)]
pub struct Config {
    pub token: String,
    mongodb_connection_string: String
}

pub async fn get_mongodb_tools() -> MongoClient {
    let mongodb_connection_string = read_user_from_file("config.json").unwrap().mongodb_connection_string;
    let mongodb_client_options = ClientOptions::parse(mongodb_connection_string).await.unwrap();
    MongoClient::with_options(mongodb_client_options).unwrap()
}

pub fn read_user_from_file<P: AsRef<Path>>(path: P) -> Result<Config, Box<dyn stderror>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let c = serde_json::from_reader(reader)?;

    Ok(c)
}