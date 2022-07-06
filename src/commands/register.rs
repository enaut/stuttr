use serenity::{
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::{channel::Message, id::ChannelId},
};

use crate::{start_syncing_of_one_meetup_group, Handler, Synchronizer};

#[command]
#[required_permissions("ADMINISTRATOR")]
pub async fn register(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    match (
        args.single_quoted::<String>(),
        args.single_quoted::<ChannelId>(),
    ) {
        (Ok(meetup_name), Ok(channel_id)) => {
            msg.reply_mention(&ctx.http, format!("Arguments accepted: Following the meetup.com group {meetup_name} the upcomming events should appear shortly.")).await?;
            println!("Registring a new synchronization for {meetup_name}");
            let data = ctx.data.read().await;
            let bot = data.get::<Handler>();
            println!("database: {:?}", bot);

            let bot = bot.expect("Failed to get the database");

            let guild_name = msg
                .guild_id
                .expect("No guild id")
                .get_preview(&ctx.http)
                .await
                .expect("No preview recieved")
                .name;
            let guild_id = msg.guild_id.expect("No guild id").0.to_string();
            let voice_channel_id = channel_id.0.to_string();

            let res = sqlx::query!(
                "INSERT INTO syncs (guild_id, server_name, meetup_group, voice_channel_id, number_of_events) VALUES (?, ?, ?, ?,?)",
                guild_id, // SQLITE does not support u64
                guild_name,
                meetup_name,
                voice_channel_id, // SQLITE does not support u64
                2
            )
            .execute(&bot.database)
            .await
            .unwrap();
            msg.reply_mention(&ctx.http, "Successfully followed")
                .await?;
            println!("{res:?}");
            println!("Inserted! {}", guild_id);
            println!("synchronizing: ");

            let sync = Synchronizer {
                guild_id,
                server_name: Some(guild_name),
                meetup_group: meetup_name,
                number_of_events: 2,
                voice_channel_id,
            };
            let context = ctx.clone();
            tokio::spawn(
                async move { start_syncing_of_one_meetup_group(sync, &context, false).await },
            );
            msg.reply_mention(&ctx.http, "Synchronizing... :hourglass_flowing_sand:")
                .await?;
            Ok(())
        }
        (_, _) => {
            msg.reply(&ctx.http, "An argument is required to run this command.")
                .await?;
            return Ok(());
        }
    }
}
