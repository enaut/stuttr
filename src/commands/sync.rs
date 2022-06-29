use std::sync::Arc;

use serde::Deserialize;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::prelude::*;
use serenity::prelude::*;
#[derive(Deserialize, Debug)]
struct Community {
    name: String,
    city: String,
    #[serde(alias = "upcomingEvents")]
    upcomming_events: UpcomingEvents,
}

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

#[derive(Deserialize, Debug)]
struct Meeting {
    title: String,
    #[serde(alias = "eventUrl")]
    event_url: String,
    status: String,
    #[serde(alias = "dateTime")]
    date_time: String,
    id: String,
}

#[command]
pub async fn sync(ctx: &Context, msg: &Message) -> CommandResult {
    let query = r#"query  {
        groupByUrlname(urlname: "rust-community-stuttgart") {
          name
          city
          upcomingEvents(input: {first:2, last:10}){
            count
            edges{
              node{
                title
                eventUrl
                status
                dateTime
                id
              }
            }
          }
        }
      }
    "#;
    let endpoint = "https://api.meetup.com/gql";
    let client = gql_client::Client::new(endpoint);

    let response = client.query::<Data>(query).await.unwrap().unwrap();

    let events: Vec<Meeting> = response
        .community
        .upcomming_events
        .meetings
        .into_iter()
        .map(|x| x.node)
        .collect();

    for event in events {
        println!("Item {:#?}", &event);
        let res = msg
            .guild_id
            .expect("Failed to get Guild")
            .create_scheduled_event(Arc::clone(&ctx.http), |e| {
                e.description(&event.title) // No Idea where the description appears
                    .start_time(
                        Timestamp::parse(&event.date_time.replace('+', ":00+")) // Starttime needs the extra :00 in front of the + sign to parse sucessfully
                            .expect("parse timestamp"),
                    )
                    // TODO get end time from api or calculate a default 2 hours...
                    .end_time(Timestamp::parse("2022-08-30T12:18:25Z").expect("parse end_time"))
                    // Thats what we see in the event
                    .name(event.title)
                    // external is if it is locally in a pub, `ScheduledEventType::Voice` if it is online in discord
                    .kind(ScheduledEventType::External)
                    // location is needed if it is external and it is displayed as link in the event.
                    .location(event.event_url)
                //TODO .image(Arc::clone(&ctx.http), AttachmentType::from("https://secure.meetupstatic.com/photos/event/4/c/9/6/clean_480019606.jpeg"))
            })
            .await;
        println!("{:?}", res);
    }

    Ok(())
}
