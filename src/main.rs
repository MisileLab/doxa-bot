use poise::serenity_prelude as serenity;

use serde::Deserialize;

use MisileLib::read_user_from_file;

use std::{fs::File, string::String};

pub struct Data {}
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Deserialize, Debug)]
struct Config {
    token: String
}

#[tokio::main]
async fn main() {
    const GUILD_IDS: [u64; 1] = [965262302775509042];
    poise::Framework::build()
        .token(read_user_from_file::<Config>(&File::open("config.json").unwrap()).unwrap().token)
        .options(poise::FrameworkOptions {
            commands: vec![
                clear_message()
            ],
            ..Default::default()
        })
        .user_data_setup(move |ctx, _ready, framework| Box::pin(async move {
            for i in GUILD_IDS {
                serenity::GuildId(i)
                .set_application_commands(ctx, |b| {
                    *b = poise::samples::create_application_commands(
                        &framework.options().commands,
                    );
                    b
                }).await.unwrap();
            };
            println!("Yeah, bot started!");
            Ok(Data {})
        }))
        .intents(serenity::GatewayIntents::non_privileged())
        .intents(serenity::GatewayIntents::MESSAGE_CONTENT)
        .intents(serenity::GatewayIntents::GUILD_PRESENCES)
        .run().await.unwrap();
}

#[poise::command(slash_command, guild_only, rename = "clear", required_permissions = "MANAGE_MESSAGES")]
pub async fn clear_message(
    ctx: Context<'_>,
    #[description = "지우고 싶은 메시지 수"]
    #[min = 2]
    #[max = 100] 
    amount: u16,
    #[description = "지우고 싶은 메시지들이 있는 채널"]
    channel: Option<serenity::Channel>
) -> Result<(), Error> {
    let channel_id = match channel {
        None => {
            ctx.channel_id()
        },
        Some(c) => {
            c.id()
        }
    };

    let http = &ctx.discord().http;
    let messages = channel_id.messages(&http, |retriever| retriever.limit(amount.into())).await?;
    match channel_id.delete_messages(http, messages).await {
        Ok(_) => {},
        Err(_) => { on_error(ctx).await; }
    };

    Ok(())
}

pub async fn on_error(ctx: Context<'_>) {
    ctx.send(|f| {
        f.content("잠시만, 이러면 안되는데.. <https://github.com/misilelab/doxa-bot/issues>에 문의해주실 수 있나요?").ephemeral(true)
    }).await.unwrap();
}
