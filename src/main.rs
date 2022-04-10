extern crate utility;

use poise::serenity_prelude as serenity;
use poise::serenity_prelude::futures::TryStreamExt;
struct Data {}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

use sha3::{Digest, Sha3_512};

use mongodb::{Client as MongoClient, options::ClientOptions};
use mongodb::bson::doc;
use mongodb::bson;
use mongodb::bson::Bson;

use bson::Document;

use serde::{Deserialize, Serialize};

use rand::{self, Rng};

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::mem::drop;
use std::string::String;
use std::error::Error as stderror;

use utility::mongoutil;

#[derive(Deserialize, Debug)]
struct Config {
    token: String,
    mongodb_connection_string: String
}

#[derive(Serialize, Deserialize)]
struct InsertStruct {
    #[serde(rename = "_id")]
    id: bson::oid::ObjectId,
    discord_id: u64,
    name: String,
    room_id: String,
}

#[derive(Serialize)]
struct SearchStruct {
    discord_id: u64
}

#[derive(Serialize, Deserialize)]
struct RoomInsertStruct {
    #[serde(rename = "_id")]
    id: bson::oid::ObjectId,
    room_id: String,
    creator_id: u64,
    name: String,
    description: String,
    category: String
}

#[derive(Serialize)]
struct RoomSearchStruct {
    room_id: String
}

#[tokio::main]
async fn main() {
    poise::Framework::build()
        .token(read_user_from_file("config.json").unwrap().token)
        .user_data_setup(move |_ctx, ready, _framework| Box::pin(async move {
            println!("{}로 로그인되었습니다.", ready.user.name);
            Ok(Data {})
        }))
        .options(poise::FrameworkOptions {
            commands: vec![
                user()
            ],
            ..Default::default()
        })
        .run().await.unwrap();
}

fn read_user_from_file<P: AsRef<Path>>(path: P) -> Result<Config, Box<dyn stderror>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let c = serde_json::from_reader(reader)?;

    Ok(c)
}

/// Vec<(content: String, description: String, inline: bool)>
fn embed_converter(embeds: Vec<(String, String, bool)>) -> serenity::CreateEmbed {
    let mut embed = serenity::CreateEmbed::default();
    embed.fields(embeds);
    return embed;
}



/// 특정 유저가 무슨 방을 들어갔는지 알 수 있는 명령어
#[poise::command(slash_command)]
async fn user(
    ctx: Context<'_>,
    #[description = "정보를 보고 싶은 유저, 안 적을 시 자기 자신이 선택됩니다."] 
    user: Option<serenity::User>
) -> Result<(), Error>{
    let mongodb_connection_string = read_user_from_file("config.json").unwrap().mongodb_connection_string;
    let mut mongodb_client_options = ClientOptions::parse(mongodb_connection_string).await.unwrap();
    mongodb_client_options.app_name = Some("doxa-bot".to_string());
    let mongodb_client = MongoClient::with_options(mongodb_client_options).unwrap();

    let collection = mongodb_client.database("doxabot").collection::<Document>("data");

    let usera = user.as_ref().unwrap_or(ctx.author());
    let search_struct = SearchStruct {
        discord_id: usera.id.0
    };

    let search_docs = mongoutil::bson_to_docs(&search_struct);
    let room_collection = mongodb_client.database("doxabot").collection::<Document>("room_data");
    let mut room_datas = vec![];
    match collection.find(search_docs, None).await {
        Ok(mut cursor) => {
            let mut search_datas = vec![];
            let results = cursor.try_next().await;
            match results {
                Ok(_) => {
                    while let Some(doc) = cursor.try_next().await.unwrap() {
                        search_datas.push(bson::from_bson::<InsertStruct>(Bson::Document(doc)).unwrap())
                    }
                    for st in &search_datas {
                        let room_search_struct = RoomSearchStruct {
                            room_id: st.room_id.clone()
                        };

                        let room_search_docs = mongoutil::bson_to_docs(&room_search_struct);

                        let room_result = match room_collection.find_one(room_search_docs, None).await {
                            Ok(res) => res,
                            Err(_) => None
                        };
                        match room_result {
                            None => {},
                            _ => {
                                room_datas.push(bson::from_bson::<RoomInsertStruct>(Bson::Document(room_result.unwrap())).unwrap());
                            }
                        };
                        drop(room_search_struct);
                    }
                    drop(search_datas);
                },
                Err(_) => {}
            }
            let mut embeds = vec![];
            for (i, i2) in room_datas.iter().enumerate() {
                let inline: bool;
                if i % 2 == 0 {
                    inline = false;
                } else {
                    inline = true;
                }
                embeds.push((format!("방 이름: {}", i2.name.clone()), format!("방 이름: {}", i2.room_id.clone()), inline));

            };
            drop(results);
            ctx.send(|f| {
                f.embeds.push(embed_converter(embeds));
                f
            }).await?;
        },
        Err(_) => {
            drop(room_collection);
            ctx.send(|f| f
                .content("그 유저의 데이터를 찾을 수 없어요.")
                .ephemeral(true)
            ).await?;
        }
    };
    Ok(())
}

#[allow(unused_variables)]
#[poise::command(slash_command)]
async fn room(ctx: Context<'_>) -> Result<(), Error> {
    // this is empty, because it is a parent slash command.
    Ok(())
}

/// 방을 만드는 명령어
#[poise::command(slash_command, rename = "create")]
async fn create_room(
    ctx: Context<'_>,
    #[description = "만들어질 방의 이름"]
    name: String,
    #[description = "만들어질 방의 설명"]
    description: Option<String>,
    #[description = "만들어질 방의 카테고리"]
    category: Option<String>
) -> Result<(), Error> {
    let mongodb_connection_string = read_user_from_file("config.json").unwrap().mongodb_connection_string;
    let mut mongodb_client_options = ClientOptions::parse(mongodb_connection_string).await.unwrap();
    mongodb_client_options.app_name = Some("doxa-bot".to_string());
    let mongodb_client = MongoClient::with_options(mongodb_client_options).unwrap();
    let collection = mongodb_client.database("doxabot").collection::<Document>("data");

    let user_id = ctx.author().id.0;
    
    let mut hasher = Sha3_512::new();
    let mut rng = rand::thread_rng();
    let hash_salt: u32 = rng.gen_range(1..4294967295);
    hasher.update(hash_salt.to_string());
    let room_id = std::str::from_utf8(&hasher.finalize()).unwrap().to_string();
    let mut insert_struct = RoomInsertStruct {
        id: bson::oid::ObjectId::new(),
        name: name,
        creator_id: user_id,
        description: description.unwrap_or("".to_string()),
        category: category.unwrap_or("".to_string()),
        room_id: room_id.clone()
    };
    let insert_docs = mongoutil::bson_to_docs(&insert_struct);
    let mut verify = false;
    while verify == false {
        let mut search_struct = RoomSearchStruct {
            room_id: room_id.clone()
        };
        let search_docs = mongoutil::bson_to_docs(&search_struct);
        let result = collection.find_one(search_docs.clone(), None).await.unwrap();
        match result {
            None => {
                verify = true;
                break;
            },
            Some(_) => {
                let mut hasher = Sha3_512::new();
                let mut rng = rand::thread_rng();
                let hash_salt: u32 = rng.gen_range(1..4294967295);
                hasher.update(hash_salt.to_string());
                let hashervalue = std::str::from_utf8(&hasher.finalize()).unwrap().to_string();
                search_struct.room_id = (&hashervalue).to_string();
                insert_struct.room_id = (&hashervalue).to_string();
                drop(hashervalue);
            }
        }
        drop(result);
        drop(search_struct);
        drop(search_docs);
    }
    let returnvalue = "등록이 되었어요.".to_string();
    drop(insert_struct);
    
    ctx.send(|f| f
        .content(returnvalue)
        .ephemeral(true)
    ).await?;
    Ok(())
}

/// 방에 들어갈 수 있는 명령어
#[poise::command(slash_command, rename = "join")]
async fn join_room(
    ctx: Context<'_>,
    #[description = "닉네임"] 
    nickname: String,
    #[description = "들어갈 방 아이디"] 
    roomid: String
) -> Result<(), Error>{
    let mongodb_connection_string = read_user_from_file("config.json").unwrap().mongodb_connection_string;
    let mut mongodb_client_options = ClientOptions::parse(mongodb_connection_string).await.unwrap();
    mongodb_client_options.app_name = Some("doxa-bot".to_string());
    let mongodb_client = MongoClient::with_options(mongodb_client_options).unwrap();

    let collection = mongodb_client.database("doxabot").collection("data");
    let user_id = ctx.id();
    let insert_struct = InsertStruct {
        id: bson::oid::ObjectId::new(),
        discord_id: user_id.clone(),
        name: nickname,
        room_id: roomid
    };
    let search_struct = SearchStruct {
        discord_id: user_id
    };
    let insert_docs = mongoutil::bson_to_docs(&insert_struct);
    let search_docs = mongoutil::bson_to_docs(&search_struct);
    let user_data = collection.find_one(search_docs, None).await.unwrap();
    let mut returnvalue = "등록이 되었어요.".to_string();
    if user_data == None {
        collection.insert_one(insert_docs, None).await.unwrap();
    } else {
        returnvalue = "이미 등록되어 있는 것 같아요.".to_string()
    }
    drop(search_struct);
    drop(insert_struct);
    
    ctx.send(|f| f
        .content(returnvalue)
        .ephemeral(true)
    ).await?;

    Ok(())
}