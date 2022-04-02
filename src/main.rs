use serenity::{
    async_trait,
    model::{
        gateway::Ready,
        id::GuildId,
        interactions::{
            application_command::{
                ApplicationCommandInteractionDataOptionValue,
                ApplicationCommandOptionType,
            },
            Interaction,
            InteractionResponseType, InteractionApplicationCommandCallbackDataFlags,
        },
    },
    prelude::*
};

#[allow(unused_imports)]
use serenity::futures::TryStreamExt;

use mongodb::{Client as MongoClient, options::ClientOptions};
use mongodb::bson::doc;
use mongodb::bson;
use mongodb::bson::Bson;

use bson::Document;

use serde::{Deserialize, Serialize};

use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::mem::drop;

struct Handler;

#[derive(Deserialize, Debug)]
struct Config {
    token: String,
    application_id: u64,
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
    name: String,
    description: String,
    category: String
}

#[derive(Serialize)]
struct RoomSearchStruct {
    room_id: String
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {

        if let Interaction::ApplicationCommand(ref command) = interaction {
            let content = match command.data.name.as_str() {
                "join" => {

                    let mongodb_connection_string = read_user_from_file("config.json").unwrap().mongodb_connection_string;
                    let mut mongodb_client_options = ClientOptions::parse(mongodb_connection_string).await.unwrap();
                    mongodb_client_options.app_name = Some("doxa-bot".to_string());
                    let mongodb_client = MongoClient::with_options(mongodb_client_options).unwrap();

                    let collection = mongodb_client.database("doxabot").collection("data");

                    let options = command
                        .data
                        .options
                        .get(0)
                        .expect("Expected user option")
                        .resolved
                        .as_ref()
                        .expect("Expected user object");

                    let user = &command.user;
                    let user_id = user.id.0;

                    if let ApplicationCommandInteractionDataOptionValue::String(stringarg) =
                        options
                    {
                        let insert_struct = InsertStruct {
                            id: bson::oid::ObjectId::new(),
                            discord_id: user_id.clone(),
                            name: stringarg.clone().to_string(),
                            room_id: "temp".to_string()
                        };
                        let search_struct = SearchStruct {
                            discord_id: user_id
                        };
                        let insert_bson = bson::to_bson(&insert_struct).unwrap();
                        let search_bson = bson::to_bson(&search_struct).unwrap();
                        let insert_docs = bson::from_bson::<Document>(insert_bson).unwrap();
                        let search_docs = bson::from_bson::<Document>(search_bson).unwrap();
                        let user_data = collection.find_one(search_docs, None).await.unwrap();
                        let mut returnvalue = "등록이 되었어요.".to_string();
                        if user_data == None {
                            collection.insert_one(insert_docs, None).await.unwrap();
                        } else {
                            returnvalue = "이미 등록되어 있는 것 같아요.".to_string()
                        }
                        drop(search_struct);
                        drop(insert_struct);
                        
                        interaction.application_command().unwrap().create_interaction_response(&ctx.http, |response| {
                            response
                                .interaction_response_data(|d| {
                                    d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL);
                                    d.content(returnvalue)
                                })
                                .kind(InteractionResponseType::ChannelMessageWithSource)
                        }).await
                    } else {
                        interaction.application_command().unwrap().create_interaction_response(&ctx.http, |response| {
                            response
                                .interaction_response_data(|d| {
                                    d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL);
                                    d.content("잘못된 유저 같아요.")
                                })
                                .kind(InteractionResponseType::ChannelMessageWithSource)
                        }).await
                    }
                },
                "userlist" => {
                    let mongodb_connection_string = read_user_from_file("config.json").unwrap().mongodb_connection_string;
                    let mut mongodb_client_options = ClientOptions::parse(mongodb_connection_string).await.unwrap();
                    mongodb_client_options.app_name = Some("doxa-bot".to_string());
                    let mongodb_client = MongoClient::with_options(mongodb_client_options).unwrap();

                    let collection = mongodb_client.database("doxabot").collection::<Document>("data");

                    let options = command
                        .data
                        .options
                        .get(0)
                        .expect("Expected user option")
                        .resolved
                        .as_ref()
                        .expect("Expected user object");

                    let user = &command.user;
                    let user_id: u64;

                    if let ApplicationCommandInteractionDataOptionValue::User(userarg, _) =
                        options
                    {
                        user_id = userarg.id.0;
                    } else {
                        user_id = user.id.0;
                    }
                    let search_struct = SearchStruct {
                        discord_id: user_id
                    };
                    
                    let search_bson = bson::to_bson(&search_struct).unwrap();
                    let search_docs = bson::from_bson::<Document>(search_bson).unwrap();
                    let room_collection = mongodb_client.database("doxabot").collection::<Document>("room_data");
                    let mut room_datas = vec![];
                    let results = match collection.find(search_docs, None).await {
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
                                        let room_search_bson = bson::to_bson(&room_search_struct).unwrap();
                                        let room_search_docs = bson::from_bson::<Document>(room_search_bson).unwrap();
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
                                if i % 2 == 0 {
                                    embeds.push((i2.name.clone(), i2.room_id.clone(), true));
                                } else {
                                    embeds.push((format!("방 이름: {}", i2.name.clone()), format!("방 아이디: {}", i2.room_id.clone()), false));
                                }
                            };
                            drop(results);
                            interaction.application_command().unwrap().create_interaction_response(&ctx.http, |response| {
                                response
                                    .interaction_response_data(|d| {
                                        d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL);
                                        d.create_embed(|e| {
                                            e.title("그 유저가 들어간 방 리스트")
                                                .fields(embeds)
                                        })
                                    })
                                    .kind(InteractionResponseType::ChannelMessageWithSource)
                            }).await
                        },
                        Err(_) => {
                            drop(room_collection);
                            interaction.application_command().unwrap().create_interaction_response(&ctx.http, |response| {
                                response
                                    .interaction_response_data(|d| {
                                        d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL);
                                        d.content("그 유저의 데이터를 찾을 수 없어요.")
                                    })
                                    .kind(InteractionResponseType::ChannelMessageWithSource)
                            }).await
                        }
                    };
                    results
                },
                "create_room" => {
                    let mongodb_connection_string = read_user_from_file("config.json").unwrap().mongodb_connection_string;
                    let mut mongodb_client_options = ClientOptions::parse(mongodb_connection_string).await.unwrap();
                    mongodb_client_options.app_name = Some("doxa-bot".to_string());
                    let mongodb_client = MongoClient::with_options(mongodb_client_options).unwrap();

                    let collection = mongodb_client.database("doxabot").collection::<Document>("data");

                    let options = command
                        .data
                        .options
                        .get(0)
                        .expect("Expected user option")
                        .resolved
                        .as_ref()
                        .expect("Expected user object");

                    let user = &command.user;
                    let user_id: u64;

                    if let ApplicationCommandInteractionDataOptionValue::User(userarg, _) =
                        options
                    {
                        user_id = userarg.id.0;
                    } else {
                        user_id = user.id.0;
                    }
                    let search_struct = SearchStruct {
                        discord_id: user_id
                    };
                    
                    let search_bson = bson::to_bson(&search_struct).unwrap();
                    let search_docs = bson::from_bson::<Document>(search_bson).unwrap();
                    let results = match collection.find(search_docs, None).await {
                        Ok(mut cursor) => {
                            let mut search_datas = vec![];
                            let results = cursor.try_next().await;
                            match results {
                                Ok(_) => {
                                    while let Some(doc) = cursor.try_next().await.unwrap() {
                                        search_datas.push(bson::from_bson::<RoomInsertStruct>(Bson::Document(doc)).unwrap())
                                    }
                                },
                                Err(_) => {}
                            }
                            drop(results);
                            interaction.application_command().unwrap().create_interaction_response(&ctx.http, |response| {
                                response
                                    .interaction_response_data(|d| {
                                        d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                                    })
                                    .kind(InteractionResponseType::ChannelMessageWithSource)
                            }).await
                        },
                        Err(_) => {
                            interaction.application_command().unwrap().create_interaction_response(&ctx.http, |response| {
                                response
                                    .interaction_response_data(|d| {
                                        d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL);
                                        d.content("그 유저의 데이터를 찾을 수 없어요.")
                                    })
                                    .kind(InteractionResponseType::ChannelMessageWithSource)
                            }).await
                        }
                    };
                    results
                }
                _ => {
                    interaction.application_command().unwrap().create_interaction_response(&ctx.http, |response| {
                        response
                            .interaction_response_data(|d| {
                                d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL);
                                d.content("핸들링되지 않은 명령어 같아요.")
                            })
                            .kind(InteractionResponseType::ChannelMessageWithSource)
                    }).await
                }
            };

            match content {
                Err(why) => {
                    println!("Cannot respond to slash command: {:#?}", why);
                },
                _ => {}
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let guild_id = GuildId(921295966143926352);

        let _commands = GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
            commands
                .create_application_command(|command| {
                    command.name("join").description("시청자 참여를 원하는 분들을 위한 명령어입니다.").create_option(|option| {
                        option
                            .name("nickname")
                            .description("게임 닉네임")
                            .kind(ApplicationCommandOptionType::String)
                            .required(true)
                    }).create_option(|option| {
                        option
                            .name("roomid")
                            .description("참여할 방의 아이디")
                            .kind(ApplicationCommandOptionType::String)
                            .required(true)
                    })
                })
                .create_application_command(|command| {
                    command.name("userlist").description("참여한 사람 리스트를 볼 수 있습니다.").create_option(|option| {
                        option
                            .name("user")
                            .description("보고 싶은 유저")
                            .kind(ApplicationCommandOptionType::User)
                            .required(false)
                    })
                })
                .create_application_command(|command| {
                    command.name("create_room").description("방을 만들 수 있습니다. (스트리머 전용)").create_option(|option|{
                        option
                            .name("name")
                            .description("방 이름")
                            .kind(ApplicationCommandOptionType::String)
                            .required(true)
                    }).create_option(|option|{
                        option
                            .name("description")
                            .description("방 설명")
                            .kind(ApplicationCommandOptionType::String)
                            .required(false)
                    }).create_option(|option|{
                        option
                            .name("category")
                            .description("방 카테고리")
                            .kind(ApplicationCommandOptionType::String)
                            .required(false)
                    })
                })
        })
        .await;

    }
}

#[tokio::main]
async fn main() {
    let config = read_user_from_file("config.json").unwrap();
    let mut client = Client::builder(&config.token)
        .event_handler(Handler)
        .application_id(config.application_id)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}

fn read_user_from_file<P: AsRef<Path>>(path: P) -> Result<Config, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let c = serde_json::from_reader(reader)?;

    Ok(c)
}
