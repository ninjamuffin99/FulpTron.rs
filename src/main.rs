use std::{collections::{HashMap, HashSet}, env, fmt::Write, sync::Arc};

use serenity::{
    client::bridge::gateway::{ShardId, ShardManager},
    framework::standard::{
        Args, CheckResult, CommandOptions, CommandResult, CommandGroup,
        DispatchError, HelpOptions, help_commands, StandardFramework,
        macros::{command, group, help, check},
    },
    model::{channel::{Channel, Message}, gateway::Ready, id::UserId},
    utils::{content_safe, ContentSafeOptions},
};

use serenity::prelude::*;

struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct CommandCounter;

impl TypeMapKey for CommandCounter {
    type Value = HashMap<String, u64>;
}

struct Handler;

impl EventHandler for Handler {
    fn message(&self, ctx: Context, msg: Message)
    {
        if msg.content == "!messageme"
        {
            let dm = msg.author.dm(&ctx, |m| {
                m.content("Hellow!");

                m
            });

            if let Err(why) = dm {
                println!("Error when direct messaging user: {:?}", why);
            }
        }

        if msg.content =="!ping"
        {

            let channel = match msg.channel_id.to_channel(&ctx)
            {
                Ok(channel) => channel,
                Err(why) => {
                    println!("Error getting channel: {:?}", why);

                    return;
                }
            };

            let response = MessageBuilder::new()
                .push("User ")
                .push_bold_safe(msg.author.name)
                .push(" used the 'ping' command in the ")
                .mention(&channel)
                .build();

            if let Err(why) = msg.channel_id.say(&ctx.http, &response) {
                println!("Error sending message: {:?}", why);
            }
            
        }
    }

    fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

fn main() {
    // Configure the client with your Discord bot token in the environment.
    let token = "NDgwOTA4ODcyNDg0MjU3ODA1.XZE6gw.0oRPB0lIYSfYgZrspkVs2XghQL8";

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let mut client = Client::new(&token, Handler).expect("Err creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start() {
        println!("Client error: {:?}", why);
    }
}
