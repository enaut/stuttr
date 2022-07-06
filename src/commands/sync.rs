use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};

use crate::sync::do_sync;

#[command]
pub async fn sync(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.expect("GuildId not found");
    if let (Ok(meetup_group_name), Ok(voice_channel_id)) = (
        args.single_quoted::<String>(),
        args.single_quoted::<ChannelId>(),
    ) {
        do_sync(
            ctx,
            guild_id,
            "ignored".to_string(),
            meetup_group_name,
            voice_channel_id,
            2,
        )
        .await
    } else {
        msg.reply(&ctx.http, "Parsing the arguments failed").await?;
        Ok(())
    }
}
