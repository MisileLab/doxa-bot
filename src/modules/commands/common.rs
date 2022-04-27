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

use std::{
    mem::drop, 
    string::String,
};

use utility::*;

use crate::modules::{utilities::doxautil::*, structs::*};

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
                .content("잠시만, 이러면 안되는데.. <https://github.com/misilelab/doxa-bot/issues>에 문의해주실 수 있나요?")
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
                room_id,
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
            let returnvalue: &str = match collection.insert_one(insert_docs, None).await {
                Ok(_) => { "방이 만들어졌어요." },
                Err(_) => { "잠시만, 이러면 안되는데.. <https://github.com/misilelab/doxa-bot/issues>에 문의해주실 수 있나요?" }
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

    let database = mongodb_client.database("doxabot");
    let collection = database.collection("data");
    let room_collection = database.collection::<Document>("room_data");
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
    let returnvalue: &str;

    match room_collection.find_one(room_search_docs, None).await.unwrap_or(None) {
        Some(_) => {
            match user_data {
                None => {
                    collection.insert_one(insert_docs, None).await.unwrap();
                    returnvalue = "등록이 되었어요."
                },
                _ => { returnvalue = "이미 등록되어 있는 것 같아요." }
            }
        },
        None => { returnvalue = "그런 아이디를 가진 방은 없는 것 같아요." }
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

    let database = mongodb_client.database("doxabot");
    let collection = database.collection::<Document>("data");
    let room_collection = database.collection::<Document>("room_data");
    let user_id = ctx.author().id.0;
    let search_struct = SearchStruct {
        discord_id: user_id,
        room_id: Some(room_id)
    };
    let room_search_struct = RoomSearchStruct { room_id };

    let search_docs = mongoutil::bson_to_docs(&search_struct);
    let room_search_docs = mongoutil::bson_to_docs(&room_search_struct);
    let user_data = collection.find_one(search_docs, None).await.unwrap();
    let returnvalue: &str;

    match room_collection.find_one(room_search_docs, None).await.unwrap_or(None) {
        Some(_) => {
            match user_data {
                None => { returnvalue = "아직 등록이 안 되어 있는 것 같아요." },
                Some(docs) => {
                    match collection.find_one_and_delete(docs, None).await {
                        Ok(_) => { returnvalue = "방에 나가는데 성공했어요!" },
                        Err(_) => { returnvalue = "잠시만, 이러면 안되는데.. <https://github.com/misilelab/doxa-bot/issues>에 문의해주실 수 있나요?" }
                    }
                }
            }
        },
        None => { returnvalue = "그런 아이디를 가진 방은 없는 것 같아요." }
    };

    drop(search_struct);
    drop(room_search_struct);
    
    ctx.send(|f| f
        .content(returnvalue)
        .ephemeral(true)
    ).await?;

    Ok(())
}

#[poise::command(slash_command)]
pub async fn delete_room(
    ctx: Context<'_>, 
    #[description = "삭제할 방 아이디"]
    room_id: u64
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
        },
        Some(_) => {
            let search_struct = RoomSearchStruct {
                room_id
            };
            let returnvalue = match collection.find_one(mongoutil::bson_to_docs(&search_struct), None).await.unwrap_or(None) {
                Some(doc) => {
                    match collection.delete_one(doc, None).await {
                        Ok(_) => "방이 삭제되었어요.",
                        Err(_) => "잠시만, 이러면 안되는데.. <https://github.com/misilelab/doxa-bot/issues>에 문의해주실 수 있나요?"
                    }
                },
                None => "그런 방은 없는 것 같아요."
            };

            drop(search_struct);
            
            ctx.send(|f| f
                .content(returnvalue)
                .ephemeral(true)
            ).await?;
        }
    }
    drop(streamer_search_struct);
    Ok(())
}

#[poise::command(slash_command)]
pub async fn streamer(_: Context<'_>) -> Result<(), Error> {
    // this is empty, because it is a parent slash command.
    Ok(())
}

/// 스트리머를 추가하는 명령어
#[poise::command(slash_command, rename = "add")]
pub async fn add_streamer(
    ctx: Context<'_>,
    #[description = "추가할 스트리머"]
    user: serenity::User
) -> Result<(), Error> {
    let mongodb_client = get_mongodb_tools().await;
    let collection = mongodb_client.database("doxabot").collection::<Document>("streamer_data");

    let search_struct = Streamer {
        id: None,
        user_id: user.id.0
    };
    let streamer_search_struct = Streamer {
        id: None,
        user_id: ctx.author().id.0
    };

    let search_docs = mongoutil::bson_to_docs(&search_struct);
    let (collection, _, streamervalue) = mongoutil::is_exist(collection, mongoutil::bson_to_docs(&streamer_search_struct)).await;
    let (collection, _, existvalue) = mongoutil::is_exist(collection, search_docs.clone()).await;
    let returnvalue: &str;

    if !streamervalue {
        ctx.send(|f| f
            .content("당신은 스트리머가 아닌 것 같아요.")
            .ephemeral(true)
        ).await?;
    } else {
        if existvalue {
            returnvalue = "이미 그 유저는 크루에 포함되어 있는 것 같아요.";
        } else {
            match collection.insert_one(search_docs, None).await {
                Ok(_) => { returnvalue = "크루에 새로운 사람이 들어왔어요!" },
                Err(_) => { returnvalue = "잠시만, 이러면 안되는데.. <https://github.com/misilelab/doxa-bot/issues>에 문의해주실 수 있나요?" }
            }
        }
    
        ctx.send(|f| f
            .content(returnvalue)
            .ephemeral(true)
        ).await?;
    }
    drop(search_struct);
    drop(streamer_search_struct);

    Ok(())
}

/// 크루에 소속되어 있는 스트리머들을 알 수 있는 명령어
#[poise::command(slash_command, rename = "list")]
pub async fn streamer_list(ctx: Context<'_>) -> Result<(), Error> {
    let mongodb_client = get_mongodb_tools().await;
    let collection = mongodb_client.database("doxabot").collection::<Document>("streamer_data");

    match collection.find(None, None).await {
        Ok(mut cursor) => {
            let mut search_datas = vec![];
            while let Some(docu) = cursor.try_next().await.unwrap() {
                search_datas.push(bson::from_bson::<Streamer>(Bson::Document(docu)).unwrap())
            }
        
            let mut embeds = vec![];
            for (i, i2) in search_datas.iter().enumerate() {
                if i2.user_id != 338902243476635650 {
                    let inline = i % 2 != 0;
                    let user_id = i2.user_id;
                    let user = match serenity::UserId(user_id).to_user(ctx.discord().http.clone()).await {
                        Ok(user) => user,
                        Err(_) => serenity::User::default()
                    };
                    embeds.push((format!("스트리머 이름: {}", user.name), format!("디스코드 아이디: {}", user_id), inline));
                }
            };
            let multiple = if embeds.is_empty() { "" } else { "들" };
            drop(search_datas);
        
            ctx.send(|f| {
                let mut embed = serenityutil::add_fields(embeds.clone());
                embed.title(format!("여기에 있는 스트리머분{}을 환영해주세요!", multiple));
                embed.description(format!("{:?}명의 스트리머가 있어요.", embeds.len()));
                f.embeds.push(embed);
                f
            }).await?;
        },
        Err(_) => {
            ctx.send(|f| {
                f.content("잠시만, 이러면 안되는데.. <https://github.com/misilelab/doxa-bot/issues>에 문의해주실 수 있나요?").ephemeral(true)
            }).await?;
        }
    };
    Ok(())
}

/// 쓸 일이 없길 바라지만, 언젠가는 써야 하는 명령어
#[poise::command(slash_command)]
pub async fn exit_streamer(
    ctx: Context<'_>,
    #[description = "추가할 스트리머"]
    user: serenity::User
) -> Result<(), Error> {
    let mongodb_client = get_mongodb_tools().await;
    let collection = mongodb_client.database("doxabot").collection::<Document>("streamer_data");

    let search_struct = Streamer {
        id: None,
        user_id: user.id.0
    };
    let streamer_search_struct = Streamer {
        id: None,
        user_id: ctx.author().id.0
    };

    let search_docs = mongoutil::bson_to_docs(&search_struct);
    let (collection, _, existvalue) = mongoutil::is_exist(collection, search_docs.clone()).await;
    let (collection, _, streamervalue) = mongoutil::is_exist(collection, mongoutil::bson_to_docs(&streamer_search_struct)).await;
    let returnvalue: &str;

    if !streamervalue {
        ctx.send(|f| f
            .content("당신은 스트리머가 아닌 것 같아요.")
            .ephemeral(true)
        ).await?;
    } else {
        if existvalue {
            match collection.delete_one(search_docs, None).await {
                Ok(_) => { returnvalue = "..." },
                Err(_) => { returnvalue = "잠시만, 이러면 안되는데.. <https://github.com/misilelab/doxa-bot/issues>에 문의해주실 수 있나요?" }
            };
        } else {
            returnvalue = "이 유저는 크루에 이미 없는 것 같아요.";
        }
    
        ctx.send(|f| f
            .content(returnvalue)
            .ephemeral(true)
        ).await?;
    }
    drop(search_struct);
    drop(streamer_search_struct);

    Ok(())
}