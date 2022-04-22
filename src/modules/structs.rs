extern crate utility;

use mongodb::bson::{self, doc};

use serde::{Deserialize, Serialize};

use std::string::String;

#[derive(Serialize, Deserialize)]
pub struct InsertStruct {
    #[serde(rename = "_id")]
    pub id: bson::oid::ObjectId,
    pub discord_id: u64,
    pub name: String,
    pub room_id: u64,
}

#[derive(Serialize)]
pub struct SearchStruct {
    pub discord_id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<u64>
}

#[derive(Serialize, Deserialize)]
pub struct RoomInsertStruct {
    #[serde(rename = "_id")]
    pub id: bson::oid::ObjectId,
    pub room_id: u64,
    pub creator_id: u64,
    pub name: String,
    pub description: String,
    pub category: String
}

#[derive(Serialize)]
pub struct RoomSearchStruct {
    pub room_id: u64
}