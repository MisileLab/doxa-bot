extern crate utility;

use poise::serenity_prelude::{
    self as serenity,
    futures::TryStreamExt
};

use mongodb::{
    bson::{
        self,
        doc,
        Bson,
        Document
    }
};

use std::{mem::drop, string::String};

use utility::*;

use crate::modules::utilities::doxautil::*;
use crate::modules::structs::*;

pub struct Data {}
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

/// 특정 유저가 무슨 방을 들어갔는지 알 수 있는 명령어
#[poise::command(slash_command)]
pub async fn user(
    ctx: Context<'_>,
    #[description = "정보를 보고 싶은 유저, 안 적을 시 자기 자신이 선택됩니다."] 
    user: Option<serenity::User>
) -> Result<(), Error>{
    let mongodb_client = get_mongodb_tools().await;

    let collection = mongodb_client.database("doxabot").collection::<Document>("data");

    let usera = user.as_ref().unwrap_or_else(|| ctx.author());
    let search_struct = SearchStruct {
        discord_id: usera.id.0,
        room_id: None
    };

    let search_docs = mongoutil::bson_to_docs(&search_struct);
    let room_collection = mongodb_client.database("doxabot").collection::<Document>("room_data");
    let mut room_datas = vec![];
    match collection.find(search_docs, None).await {
        Ok(mut cursor) => {
            let mut search_datas = vec![];
            while let Some(docu) = cursor.try_next().await.unwrap() {
                search_datas.push(bson::from_bson::<InsertStruct>(Bson::Document(docu)).unwrap())
            }
            for st in &search_datas {
                let room_search_struct = RoomSearchStruct {
                    room_id: st.room_id
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
            let mut embeds = vec![];
            for (i, i2) in search_datas.iter().enumerate() {
                let inline = i % 2 != 0;
                embeds.push((format!("방 이름: {}", room_datas[i].name.clone()), format!("방 아이디: {}", i2.room_id.clone()), inline));
            };
            drop(search_datas);

            ctx.send(|f| {
                let mut embed = serenityutil::add_fields(embeds.clone());
                embed.title(format!("{}님의 데이터", usera.name));
                embed.description(format!("{:?}개의 데이터가 있습니다.", embeds.len()));
                f.embeds.push(embed);
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

#[poise::command(slash_command)]
pub async fn room(_: Context<'_>) -> Result<(), Error> {
    // this is empty, because it is a parent slash command.
    Ok(())
}

/// 방을 만드는 명령어
#[poise::command(slash_command, rename = "create")]
pub async fn create_room(
    ctx: Context<'_>,
    #[description = "만들어질 방의 이름"]
    name: String,
    #[description = "만들어질 방의 설명"]
    description: Option<String>,
    #[description = "만들어질 방의 카테고리"]
    category: Option<String>
) -> Result<(), Error> {
    let mongodb_client = get_mongodb_tools().await;
    let database = mongodb_client.database("doxabot");
    let collection = database.collection::<Document>("room_data");
    let streamer_collection = database.collection::<Document>("streamer_data");

    let user_id = ctx.author().id.0;

    let streamer_search_struct = Streamer {
        id: None,
        user_id
    };

    match streamer_collection.find_one(mongoutil::bson_to_docs(&streamer_search_struct), None).await.unwrap() {
        None => {
            ctx.send(|f| f
                .content("스트리머가 아닌 것 같아요.")
                .ephemeral(true)
            ).await?;
            Ok(())
        },
        Some(_) => {
            let mut room_id: u64 = 0;
            let mut insert_struct = RoomInsertStruct {
                id: bson::oid::ObjectId::new(),
                name,
                creator_id: user_id,
                description: description.unwrap_or_else(|| "".to_string()),
                category: category.unwrap_or_else(|| "".to_string()),
                room_id
            };
            let mut verify = false;
            while !verify {
                let search_struct = RoomSearchStruct {
                    room_id: insert_struct.room_id
                };
                let search_docs = mongoutil::bson_to_docs(&search_struct);
                let result = collection.find_one(search_docs.clone(), None).await.unwrap();
                match result {
                    None => verify = true,
                    Some(_) => {
                        room_id += 1;
                        insert_struct.room_id = room_id;
                    }
                }
                drop(result);
                drop(search_struct);
                drop(search_docs);
            }
            let insert_docs = mongoutil::bson_to_docs(&insert_struct);
            let returnvalue = match collection.insert_one(insert_docs, None).await {
                Ok(_) => { "방이 만들어졌어요.".to_string() },
                Err(_) => { "제대로 만들어지지 않은 것 같아요.".to_string() }
            };
            drop(insert_struct);
            drop(streamer_search_struct);
            
            ctx.send(|f| f
                .content(returnvalue)
                .ephemeral(true)
            ).await?;
            Ok(())
        }
    }
}

/// 방에 들어갈 수 있는 명령어
#[poise::command(slash_command, rename = "join")]
pub async fn join_room(
    ctx: Context<'_>,
    #[description = "닉네임"] 
    nickname: String,
    #[description = "들어갈 방 아이디"] 
    roomid: u64
) -> Result<(), Error>{
    let mongodb_client = get_mongodb_tools().await;

    let collection = mongodb_client.database("doxabot").collection("data");
    let room_collection = mongodb_client.database("doxabot").collection::<Document>("room_data");
    let user_id = ctx.author().id.0;
    let insert_struct = InsertStruct {
        id: bson::oid::ObjectId::new(),
        discord_id: user_id,
        name: nickname,
        room_id: roomid
    };
    let search_struct = SearchStruct {
        discord_id: user_id,
        room_id: Some(roomid)
    };
    let room_search_struct = RoomSearchStruct {
        room_id: roomid
    };

    let insert_docs = mongoutil::bson_to_docs(&insert_struct);
    let search_docs = mongoutil::bson_to_docs(&search_struct);
    let room_search_docs = mongoutil::bson_to_docs(&room_search_struct);
    let user_data = collection.find_one(search_docs, None).await.unwrap();
    let returnvalue: String;

    match room_collection.find_one(room_search_docs, None).await.unwrap_or(None) {
        Some(_) => {
            match user_data {
                None => {
                    collection.insert_one(insert_docs, None).await.unwrap();
                    returnvalue = "등록이 되었어요.".to_string()
                },
                _ => { returnvalue = "이미 등록되어 있는 것 같아요.".to_string() }
            }
        },
        None => { returnvalue = "그런 아이디를 가진 방은 없는 것 같아요.".to_string() }
    };

    drop(search_struct);
    drop(insert_struct);
    drop(room_search_struct);
    
    ctx.send(|f| f
        .content(returnvalue)
        .ephemeral(true)
    ).await?;

    Ok(())
}

/// 방을 나가는 명령어
#[poise::command(slash_command, rename = "exit")]
pub async fn exit_room(
    ctx: Context<'_>,
    #[description = "나갈 방 아이디"]
    room_id: u64
) -> Result<(), Error> {
    let mongodb_client = get_mongodb_tools().await;

    let collection = mongodb_client.database("doxabot").collection::<Document>("data");
    let room_collection = mongodb_client.database("doxabot").collection::<Document>("room_data");
    let user_id = ctx.author().id.0;
    let search_struct = SearchStruct {
        discord_id: user_id,
        room_id: Some(room_id)
    };
    let room_search_struct = RoomSearchStruct { room_id };

    let search_docs = mongoutil::bson_to_docs(&search_struct);
    let room_search_docs = mongoutil::bson_to_docs(&room_search_struct);
    let user_data = collection.find_one(search_docs, None).await.unwrap();
    let returnvalue: String;

    match room_collection.find_one(room_search_docs, None).await.unwrap_or(None) {
        Some(_) => {
            match user_data {
                None => { returnvalue = "아직 등록이 안 되어 있는 것 같아요.".to_string() },
                Some(docs) => {
                    match collection.find_one_and_delete(docs, None).await {
                        Ok(_) => { returnvalue = "방에 나가는데 성공했어요!".to_string(); },
                        Err(_) => { returnvalue = "뭔가 잘못된 것 같아요.".to_string() }
                    }
                }
            }
        },
        None => { returnvalue = "그런 아이디를 가진 방은 없는 것 같아요.".to_string() }
    };

    drop(search_struct);
    drop(room_search_struct);
    
    ctx.send(|f| f
        .content(returnvalue)
        .ephemeral(true)
    ).await?;

    Ok(())
}