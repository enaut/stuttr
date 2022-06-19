use std::io::Read;

use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::prelude::*;
use serenity::prelude::*;

use web_ical::Calendar;

#[command]
pub async fn list(ctx: &Context, msg: &Message) -> CommandResult {
    println!("Listing some ical events");

    let mut file =
        std::fs::File::open("/home/dietrich/Downloads/ics").expect("Failed to open file");
    let mut data = String::new();
    file.read_to_string(&mut data).expect("Failed to read Data");

    println!("{}", data);

    let events = Calendar::new_from_data(&data).expect("Failed to parse events");

    println!("{:?}", events.events.len());
    println!("{:?}", events.version);

    for line in events.events {
        println!("Item {:#?}", &line.summary);
        msg.channel_id.say(&ctx.http, "test").await?;
    }

    Ok(())
}
