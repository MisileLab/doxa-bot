extern crate utility;

use poise::serenity_prelude as serenity;

use crate::modules::commands::streamer::{Context, Error};
use crate::on_error;

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