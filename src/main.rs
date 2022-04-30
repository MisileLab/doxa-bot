extern crate utility;

mod modules;

use poise::serenity_prelude as serenity;

use modules::utilities::doxautil::read_user_from_file;
use modules::commands::{streamer::*, common::*};

#[tokio::main]
async fn main() {
    const GUILD_IDS: [u64; 1] = [965262302775509042];
    poise::Framework::build()
        .token(read_user_from_file("config.json").unwrap().token)
        .options(poise::FrameworkOptions {
            commands: vec![
                user(),
                poise::Command {
                    subcommands: vec![
                        create_room(),
                        join_room(),
                        exit_room(), 
                        delete_room()
                    ],
                    ..room()
                },
                poise::Command {
                    subcommands: vec![
                        add_streamer(),
                        streamer_list(),
                    ],
                    ..streamer()
                },
                clear_message()
                // exit_streamer()
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