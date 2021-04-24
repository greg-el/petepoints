use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use regex::Regex;
use serenity::model::id::UserId;
use serenity::utils::{parse_mention, MessageBuilder};
use serenity::{
    async_trait,
    client::bridge::gateway::GatewayIntents,
    framework::standard::{
        macros::{command, group, hook},
        CommandResult, StandardFramework,
    },
    model::channel::Message,
    model::gateway::Ready,
    prelude::*,
};
use serde_json;
use tokio::sync::RwLock;

#[macro_use]
extern crate dotenv_codegen;
#[macro_use]
extern crate lazy_static;


struct PetePoints;

impl PetePoints {
    fn parse_command(msg: &Message) -> ParsedMessage {
        let parsed_msg: Vec<&str> = msg.content.split_whitespace().collect();
        ParsedMessage {
            amount: parsed_msg[1].parse::<i64>().unwrap(),
            discord_id: parse_mention(parsed_msg[2]).unwrap(),
        }
    }

    async fn write_points(ctx: &Context, parsed_msg: &ParsedMessage) {
        let points_lock = {
            let data_read = ctx.data.read().await;
            data_read
                .get::<PetePoints>()
                .expect("Expected PetePoints in TypeMap")
                .clone()
        };

        {
            let mut points = points_lock.write().await;
            let entry = points
                .entry(parsed_msg.discord_id)
                .or_insert(parsed_msg.amount);
            *entry += parsed_msg.amount;
        }
    }

    async fn update_user_points(ctx: &Context, msg: &Message) {
        let parsed_msg = PetePoints::parse_command(&msg);
        PetePoints::write_points(&ctx, &parsed_msg).await;
        let amount = PetePoints::get_user_points(&ctx, &parsed_msg).await;
        let message = MessageBuilder::new()
            .mention(&UserId(parsed_msg.discord_id))
            .push(format!(" has {} Pete Points!", amount))
            .build();

        msg.reply(&ctx, message)
            .await
            .expect("Error reading Pete Points for user");

        PetePoints::write_points_to_file(&ctx).await;
    }

    async fn get_user_points(ctx: &Context, parsed_msg: &ParsedMessage) -> i64 {
        {
            let data_read = ctx.data.read().await;
            let pete_points_lock = data_read
                .get::<PetePoints>()
                .expect("Expected PetePoints in TypeMap")
                .clone();
            let pete_points = pete_points_lock.read().await;
            pete_points.get(&parsed_msg.discord_id).map_or(0, |x| *x)
        }
    }

    async fn write_points_to_file(ctx: &Context) {
        let data_read = ctx.data.read().await;
        let pete_points_lock = data_read
            .get::<PetePoints>()
            .expect("Expected PetePoints in TypeMap")
            .clone();
        let pete_points = pete_points_lock.read().await;
        let to_write = serde_json::to_string(&pete_points.clone());
        if let Ok(write_data) = to_write {
            tokio::fs::write("pete-points.txt", write_data)
                .await
                .expect("Failed to write to file");
        }
    }
}

impl TypeMapKey for PetePoints {
    type Value = Arc<RwLock<HashMap<u64, i64>>>;
}

struct ParsedMessage {
    amount: i64,
    discord_id: u64,
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, _rdy: Ready) {
        println!("Ready!");
    }
}

#[group]
#[description("General Pete Points Commands")]
#[commands(pp)]
struct General;

#[hook]
async fn before(ctx: &Context, msg: &Message, cmd: &str) -> bool {
    if cmd == "pp" {
        let message_split: Vec<&str> = msg.content.split_whitespace().collect();
        return match message_split[1] {
            "show" => true,
            _ => {
                lazy_static! {
                    static ref ADD_POINT_PATTERN: Regex =
                        Regex::new(r"(?i)!pp\s\+\d+\s<@!\d+>").unwrap();
                }
                match ADD_POINT_PATTERN.is_match(&msg.content) {
                    true => true,
                    false => {
                        msg.reply(
                            &ctx,
                            "That is not the valid command to give someone Pete Points.",
                        )
                        .await
                        .expect("There was a problem replying to the message.");
                        false
                    }
                }
            }
        };
    }
    true
}

#[hook]
async fn unrecognised_command(_ctx: &Context, _msg: &Message, cmd: &str) {
    println!("Command not known: {:?}", cmd);
}

#[tokio::main]
async fn main() {
    // Discord handling
    let token = dotenv!("TOKEN");
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!"))
        .before(before)
        .unrecognised_command(unrecognised_command)
        .group(&GENERAL_GROUP);
    let mut client = Client::builder(token)
        .event_handler(Handler)
        .framework(framework)
        .intents(GatewayIntents::all())
        .await
        .expect("Error creating client");

    // File handling
    if Path::new("pete-points.txt").exists() {
        let data_read = tokio::fs::read_to_string("pete-points.txt").await;
        if let Ok(data_result) = data_read {
            let points_hm: HashMap<u64, i64> = serde_json::from_str(&data_result).unwrap();
            {
                let mut data = client.data.write().await;
                data.insert::<PetePoints>(Arc::new(RwLock::new(points_hm)))
            }
        }
    } else {
        tokio::fs::File::create("pete-points.txt")
            .await
            .expect("Panicked while creating pete-points.txt");
        {
            let mut data = client.data.write().await;
            data.insert::<PetePoints>(Arc::new(RwLock::new(HashMap::new())));
        }
    };

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}

#[command]
async fn pp(ctx: &Context, msg: &Message) -> CommandResult {
    let message_split: Vec<&str> = msg.content.split_whitespace().collect();
    match message_split[1] {
        "show" => show_all_pete_points(&ctx, &msg).await,
        _ => PetePoints::update_user_points(&ctx, &msg).await,
    };
    Ok(())
}


async fn show_all_pete_points(ctx: &Context, msg: &Message) {
    {
        let data_read = ctx.data.read().await;
        let pete_points_lock = data_read
            .get::<PetePoints>()
            .expect("Expected PetePoints in TypeMap")
            .clone();
        let guild = match msg.guild(&ctx.cache).await {
            Some(g) => g,
            None => panic!("No guild"),
        };
        let pete_points = pete_points_lock.read().await;
        let mut output = MessageBuilder::new();
        for (user, points) in pete_points.clone() {
            let user_name = match guild.members.get(&UserId(user)) {
                Some(u) => u.display_name(),
                None => panic!("The user is missing"),
            };
            output.push(format!("{}: {}\n", user_name, points));
        }
        output.build();
        msg.reply(&ctx.http, output)
            .await
            .expect("There was a problem replying to the message.");
    }
}
