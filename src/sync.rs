use std::{sync::Arc, time::Duration};

use gql_client::GraphQLError;
use serde::Deserialize;
use serenity::{framework::standard::CommandResult, model::prelude::*, prelude::*};
use tokio::time::Instant;
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct Community {
    name: String,
    city: String,
    #[serde(alias = "upcomingEvents")]
    upcomming_events: UpcomingEvents,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct UpcomingEvents {
    count: usize,
    #[serde(alias = "edges")]
    meetings: Vec<Node>,
}

#[derive(Deserialize, Debug)]
struct Node {
    node: Meeting,
}

#[derive(Deserialize, Debug)]
struct Data {
    #[serde(alias = "groupByUrlname")]
    community: Community,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct Meeting {
    title: String,
    #[serde(alias = "eventUrl")]
    event_url: String,
    description: String,
    status: String,
    #[serde(alias = "dateTime")]
    date_time: String,
    duration: String, // TODO Weird format need better understanding
    id: String,
    #[serde(alias = "isOnline")]
    is_online: bool,
}

#[derive(Deserialize)]
pub(crate) struct Synchronizer {
    pub(crate) guild_id: String,
    pub(crate) server_name: Option<String>,
    pub(crate) meetup_group: String,
    pub(crate) number_of_events: i64,
    pub(crate) voice_channel_id: String,
}

pub async fn do_sync(
    ctx: &Context,
    guild_id: GuildId,
    guild_name: String,
    meetup_group_name: String,
    voice_channel_id: ChannelId,
    number_of_events_to_sync: usize,
) -> CommandResult {
    let meetings = query_meetings(meetup_group_name, number_of_events_to_sync)
        .await
        .expect("Failed to query meetings from meetup");
    let events = get_events(ctx, guild_id).await?;
    for meeting in meetings {
        if events.iter().all(|x| {
            if let Some(d) = x.description.as_ref() {
                !d.contains(&meeting.event_url)
            } else {
                true
            }
        }) {
            println!(
                "Creating meeting {:#?} in server {guild_name}",
                &meeting.title
            );
            let sch_event = create_event(ctx, guild_id, meeting, voice_channel_id).await?;
            println!("Event created: {} ({})", sch_event.name, sch_event.id);
        } else {
            println!("Event {} already existing", meeting.event_url);
        }
    }

    Ok(())
}

async fn get_events(
    ctx: &Context,
    guild_id: GuildId,
) -> Result<Vec<ScheduledEvent>, serenity::Error> {
    let events = ctx.http.get_scheduled_events(guild_id.0, false).await?;
    Ok(events)
}

async fn create_event(
    ctx: &Context,
    guild_id: GuildId,
    meeting: Meeting,
    voice_channel_id: ChannelId,
) -> Result<ScheduledEvent, serenity::Error> {
    guild_id
        .create_scheduled_event(Arc::clone(&ctx.http), |e| {
            let description = format!(
                "{:.300}…\n(source: {})",
                &meeting.description, &meeting.event_url
            );
            let start_time = Timestamp::parse(&meeting.date_time.replace('+', ":00+"))
                .expect("Failed to parse start time"); // Starttime needs the extra :00 in front of the + sign to parse sucessfully
            let e = e
                .description(&description) // No Idea where the description appears
                .start_time(start_time)
                // TODO get end time from api or calculate a default 2 hours...
                .end_time(
                    Timestamp::from_unix_timestamp(
                        start_time.unix_timestamp() + (3600 * 2/* TODO: Use duration here */),
                    )
                    .expect("failed to parse duration"),
                )
                // Thats what we see in the event
                .name(meeting.title);
            if meeting.is_online {
                // Create an online Meeting
                e.kind(ScheduledEventType::Voice)
                    .channel_id(voice_channel_id.0)
            } else {
                // Create an physical Meeting
                e
                    // external is if it is locally in a pub, `ScheduledEventType::Voice` if it is online in discord
                    .kind(ScheduledEventType::External)
                    // location is needed if it is external and it is displayed as link in the event.
                    .location(meeting.event_url)
            }
            //TODO .image(Arc::clone(&ctx.http), AttachmentType::from("https://secure.meetupstatic.com/photos/event/4/c/9/6/clean_480019606.jpeg"))
        })
        .await
        .map_err(|e| {
            println!("Failed with {}", &e);
            e
        })
}

async fn query_meetings(
    meetup_group_name: String,
    number_of_events_to_sync: usize,
) -> Result<Vec<Meeting>, GraphQLError> {
    let query = format!(
        r#"query  {{
    groupByUrlname(urlname: "{meetup_group_name}") {{
      name
      city
      upcomingEvents(input: {{first:{number_of_events_to_sync}, last:10}}){{
        count
        edges{{
          node{{
            title
            description
            eventUrl
            status
            dateTime
            duration
            id
            isOnline
          }}
        }}
      }}
    }}
  }}
"#
    );
    println!("{query}");
    let endpoint = "https://api.meetup.com/gql";
    let client = gql_client::Client::new(endpoint);

    let response = client.query::<Data>(&query).await?;

    let events: Vec<Meeting> = response
        .into_iter()
        .flat_map(|e| {
            e.community
                .upcomming_events
                .meetings
                .into_iter()
                .map(|x| x.node)
        })
        .collect();
    Ok(events)
}

pub(crate) async fn start_syncing_of_one_meetup_group(
    s: Synchronizer,
    ctx: &Context,
    initial_wait: bool,
) {
    let start = if initial_wait {
        let offset = rand::random::<u8>() as u64 * 3;
        println!("Waiting for {offset} seconds");
        Instant::now() + Duration::from_secs(offset)
    } else {
        Instant::now()
    };
    let mut interval = tokio::time::interval_at(start, Duration::from_secs(900));

    loop {
        interval.tick().await;
        let guild_id = ctx
            .http
            .get_guild(s.guild_id.parse().expect("Failed to parse guild_id"))
            .await
            .expect("Failed to find guild")
            .id;
        let channel_id = match ctx
            .http
            .get_channel(
                s.voice_channel_id
                    .parse()
                    .expect("Failed to parse channel_id"),
            )
            .await
            .expect("Failed to find guild")
        {
            Channel::Guild(GuildChannel { id, .. })
            | Channel::Private(PrivateChannel { id, .. })
            | Channel::Category(ChannelCategory { id, .. }) => id,
            _ => unreachable!("No Idea how this could be reached"),
        };
        let num_of_events_to_sync = s.number_of_events as usize;
        let guild_name = s
            .server_name
            .clone()
            .unwrap_or_else(|| "No name".to_string());
        do_sync(
            ctx,
            guild_id,
            guild_name,
            s.meetup_group.clone(),
            channel_id,
            num_of_events_to_sync,
        )
        .await
        .expect("Failed to sync");
    }
}
