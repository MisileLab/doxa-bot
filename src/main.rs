extern crate utility;

mod modules;

use poise::serenity_prelude as serenity;

use modules::utilities::doxautil::read_user_from_file;
use modules::commands::{common::*};

#[tokio::main]
async fn main() {
    let guild_id: u64 = 921295966143926352;
    poise::Framework::build()
        .token(read_user_from_file("config.json").unwrap().token)
        .options(poise::FrameworkOptions {
            commands: vec![
                user(),
                poise::Command {
                    subcommands: vec![
                        create_room(),
                        join_room(),
                    ],
                    ..room()
                },
            ],
            ..Default::default()
        })
        .user_data_setup(move |ctx, _ready, framework| Box::pin(async move {
            serenity::GuildId(guild_id)
            .set_application_commands(ctx, |b| {
                *b = poise::samples::create_application_commands(
                    &framework.options().commands,
                );
                b
            }).await.unwrap();
            println!("Yeah, bot started!");
            Ok(Data {})
        }))
        .intents(serenity::GatewayIntents::non_privileged())
        .intents(serenity::GatewayIntents::MESSAGE_CONTENT)
        .intents(serenity::GatewayIntents::GUILD_PRESENCES)
        .run().await.unwrap();
}