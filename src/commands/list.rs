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
pub async fn list(ctx: &Context, msg: &Message) -> CommandResult {
    let query = r#"query  {
        groupByUrlname(urlname: "rust-community-stuttgart") {
          name
          city   
          upcomingEvents(input: {first:3, last:10}){
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

    for line in events {
        println!("Item {:#?}", &line);
        msg.channel_id
            .say(&ctx.http, format!("Item {:#?}", line))
            .await?;
    }

    Ok(())
}
