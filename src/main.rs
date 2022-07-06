//! Requires the 'framework' feature flag be enabled in your project's
//! `Cargo.toml`.
//!
//! This can be enabled by specifying the feature in the dependency section:
//!
//! ```toml
//! [dependencies.serenity]
//! git = "https://github.com/serenity-rs/serenity.git"
//! features = ["framework", "standard_framework"]
//! ```
mod commands;
mod sync;

use std::{collections::HashSet, env, sync::Arc, time::Duration};

use serde::Deserialize;
use serenity::model::channel::{Channel, ChannelCategory, GuildChannel, PrivateChannel};
use serenity::{
    async_trait,
    client::bridge::gateway::ShardManager,
    framework::{standard::macros::group, StandardFramework},
    http::Http,
    model::{event::ResumedEvent, gateway::Ready},
    prelude::*,
};
use tokio::time::Instant;
use tracing::{error, info};

use crate::sync::{start_syncing_of_one_meetup_group, Synchronizer};
use crate::{
    commands::{register::*, sync::*},
    sync::do_sync,
};

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

#[derive(Debug, Clone)]
struct Handler {
    database: sqlx::SqlitePool,
}

impl TypeMapKey for Handler {
    type Value = Handler;
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        let data = ctx.data.read().await;
        let bot = data.get::<Handler>();

        let res: Vec<Synchronizer> = sqlx::query_as!(Synchronizer, "SELECT * from syncs")
            .fetch_all(&bot.expect("A database is not available").database)
            .await
            .expect("Failed to query the channels to sync");

        info!("Connected as {}", ready.user.name);
        for s in res {
            let ctx = ctx.clone();
            tokio::spawn(async move { start_syncing_of_one_meetup_group(s, &ctx, true).await });
        }
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }
}

#[group]
#[commands(sync, register)]
struct General;

#[tokio::main]
async fn main() {
    // This will load the environment variables located at `./.env`, relative to
    // the CWD. See `./.env-sample` for an example on how to structure this.
    dotenv::dotenv().expect("Failed to load .env file");

    // Initialize the logger to use environment variables.
    //
    // In this case, a good default is setting the environment variable
    // `RUST_LOG` to `debug`.
    tracing_subscriber::fmt::init();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    // Initiate a connection to the database file, creating the file if required.
    let default_file = "database.sqlite".to_owned();
    let database_url = env::var("DATABASE_URL").unwrap_or(default_file);
    let database_url = database_url.trim_start_matches("sqlite:");
    let database = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(database_url)
                .create_if_missing(true),
        )
        .await
        .expect("Couldn't connect to database");
    sqlx::migrate!()
        .run(&database)
        .await
        .expect("Failed to apply migrations");

    let http = Http::new(&token);

    // We will fetch your bot's owners and id
    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    // Create the framework
    let framework = StandardFramework::new()
        .configure(|c| c.owners(owners).prefix("~"))
        .group(&GENERAL_GROUP);

    let bot = Handler { database };

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_SCHEDULED_EVENTS;
    let mut client = Client::builder(&token, intents)
        .framework(framework)
        .event_handler(bot.clone())
        .await
        .expect("Error creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<Handler>(bot.clone());
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
    }

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}
