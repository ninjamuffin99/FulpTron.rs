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
    fn ready(&self, _: Context, ready: Ready)
    {
        println!("{} is connected!", ready.user.name);
    }
}
group!({
    name: "general",
    options: {},
    commands: [about, say, commands, ping, some_long_command]
});

group!({
    name: "emoji",
    options: {
        prefixes: ["emoji", "em"],
        description: "A group with commands providing an emoji as response.",
        default_command: bird,
    },
    commands: [],
});

group!({
    name:"math",
    options: {
        prefix: "math",
    },
    commands: [multiply],
});

group!({
    name: "owner",
    options: {
        owners_only: true,
        only_in: "guilds",
        checks: [Admin],
    },
    commands: [am_i_admin, slow_mode],
});

#[help]
#[individual_command_tip = 
"Hello! こんにちは！Hola! Bonjour! 您好!\n\
If you want more information about a specific command, just pass the command as argument."]
#[command_not_found_text = "Could not find: '{}'"]
#[max_levenshtein_distance(3)]
#[indention_prefix = "+"]
#[lacking_permissions = "Hide"]
#[lacking_role = "Nothing"]
#[wrong_channel = "Strike"]
fn my_help(
    context: &mut Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>
) -> CommandResult {
    help_commands::with_embeds(context, msg, args, help_options, groups, owners)
}

fn main() {
    // Configure the client with your Discord bot token in the environment.
    let token = "NDgwOTA4ODcyNDg0MjU3ODA1.XZE6gw.0oRPB0lIYSfYgZrspkVs2XghQL8";

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let mut client = Client::new(&token, Handler).expect("Err creating client");

    {
        let mut data = client.data.write();
        data.insert::<CommandCounter>(HashMap::default());
        data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));
    }

    let (owners, bot_id) = match client.cache_and_http.http.get_current_application_info() {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        },
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    client.with_framework(
            StandardFramework::new()
            .configure(|c| c
                .with_whitespace(true)
                .on_mention(Some(bot_id))
                .prefix("fulp")
                .delimiters(vec![", ", ","])
                .owners(owners))
            .before(|ctx, msg, command_name| {
                println!("Got command '{}' by user '{}'",
                    command_name,
                    msg.author.name);
                
                let mut data = ctx.data.write();
                let counter = data.get_mut::<CommandCounter>().expect("Expected CommandCOunter in ShareMap.");
                let entry = counter.entry(command_name.to_string()).or_insert(0);
                *entry += 1;

                true
            })
            .after(|_, _, command_name, error| {
                match error {
                    Ok(()) => println!("Processed command '{}'", command_name),
                    Err(why) => println!("Command '{}' returned error {:?}", command_name, why),
                }
            })
            .unrecognised_command(|_, _, unknown_command_name| {
                println!("Could not find command named '{}'", unknown_command_name);
            })
            .normal_message(|_, message| {
                println!("Message is not a command '{}'", message.content);
            })
            .on_dispatch_error(|ctx, msg, error| {
                if let DispatchError::Ratelimited(seconds) = error {
                    let _ = msg.channel_id.say(&ctx.http, &format!("Try this again in {} seconds.", seconds));
                }
            })
            .help(&MY_HELP)
        // Can't be used more than once per 5 seconds:
        .bucket("emoji", |b| b.delay(5))
        // Can't be used more than 2 times per 30 seconds, with a 5 second delay:
        .bucket("complicated", |b| b.delay(5).time_span(30).limit(2))
        // The `group!` macro generates `static` instances of the options set for the group.
        // They're made in the pattern: `#name_GROUP` for the group instance and `#name_GROUP_OPTIONS`.
        // #name is turned all uppercase
        .group(&GENERAL_GROUP)
        .group(&EMOJI_GROUP)
        .group(&MATH_GROUP)
        .group(&OWNER_GROUP)
    );


    // Starts the client
    if let Err(why) = client.start() {
        println!("Client error: {:?}", why);
    }
}

#[command]
#[bucket = "complicated"]
fn commands(ctx: &mut Context, msg: &Message) -> CommandResult {
    let mut contents = "Commands used:\n".to_string();

    let data = ctx.data.read();
    let counter = data.get::<CommandCounter>().expect("Expected COmmandCOunter in ShareMap.");

    for (k, v) in counter {
        let _ = write!(contents, "- {name}: {amount}\n", name=k, amount=v);
    }

    if let Err(why) = msg.channel_id.say(&ctx.http, &contents) {
        println!("Error sending message: {:?}", why);
    }

    Ok(())
}
#[command]
fn say(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let settings = if let Some(guild_id) = msg.guild_id {
        ContentSafeOptions::default()
            .clean_channel(false)
            .display_as_member_from(guild_id)
    } else {
        ContentSafeOptions::default()
            .clean_channel(false)
            .clean_role(false)
    };

    let content = content_safe(&ctx.cache, &args.rest(), &settings);

    if let Err(why) = msg.channel_id.say(&ctx.http, &content) {
        println!("Error sending message: {:?}", why);
    }

    Ok(())
}

#[check]
#[name = "Owner"]
fn owner_check(_: &mut Context, msg: &Message, _: &mut Args, _: &CommandOptions) -> CheckResult {
    // replace 7 with whatever user ID
    (msg.author.id == 169167794649169920).into()
}

#[check]
#[name = "Admin"]
#[check_in_help(true)]
#[display_in_help(true)]
fn admin_check(ctx: &mut Context, msg: &Message, _: &mut Args, _: &CommandOptions) -> CheckResult {
    if let Some(member) = msg.member(&ctx.cache) {
        if let Ok(permissions) = member.permissions(&ctx.cache) {
            return permissions.administrator().into();
        }
    }

    false.into()
}

#[command]
fn some_long_command(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    if let Err(why) = msg.channel_id.say(&ctx.http, &format!("Arguments: {:?}", args.rest())) {
        println!("Error sending message: {:?}", why);
    }

    Ok(())
}

#[command]
// Limits the usage of this command to roles named:
#[allowed_roles("mods", "ultimate neko")]
fn about_role(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let potential_role_name = args.rest();

    if let Some(guild) = msg.guild(&ctx.cache) {
        // `role_by_name()` allows us to attempt attaining a reference to a role
        // via its name.
        if let Some(role) = guild.read().role_by_name(&potential_role_name) {
            if let Err(why) = msg.channel_id.say(&ctx.http, &format!("Role-ID: {}", role.id)) {
                println!("Error sending message: {:?}", why);
            }

            return Ok(());
        }
    }

    if let Err(why) = msg.channel_id.say(&ctx.http, format!("Could not find role named: {:?}", potential_role_name)) {
        println!("Error sending message: {:?}", why);
    }

    Ok(())
}

#[command]
// Lets us also call `~math *` instead of just `~math multiply`.
#[aliases("*")]
fn multiply(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let first = args.single::<f64>()?;
    let second = args.single::<f64>()?;

    let res = first * second;

    if let Err(why) = msg.channel_id.say(&ctx.http, &res.to_string()) {
        println!("Err sending product of {} and {}: {:?}", first, second, why);
    }

    Ok(())
}

#[command]
fn about(ctx: &mut Context, msg: &Message) -> CommandResult {
    if let Err(why) = msg.channel_id.say(&ctx.http, "This is a small test-bot! : )") {
        println!("Error sending message: {:?}", why);
    }

    Ok(())
}

#[command]
fn latency(ctx: &mut Context, msg: &Message) -> CommandResult {
    // The shard manager is an interface for mutating, stopping, restarting, and
    // retrieving information about shards.
    let data = ctx.data.read();

    let shard_manager = match data.get::<ShardManagerContainer>() {
        Some(v) => v,
        None => {
            let _ = msg.reply(&ctx, "There was a problem getting the shard manager");

            return Ok(());
        },
    };

    let manager = shard_manager.lock();
    let runners = manager.runners.lock();

    // Shards are backed by a "shard runner" responsible for processing events
    // over the shard, so we'll get the information about the shard runner for
    // the shard this command was sent over.
    let runner = match runners.get(&ShardId(ctx.shard_id)) {
        Some(runner) => runner,
        None => {
            let _ = msg.reply(&ctx,  "No shard found");

            return Ok(());
        },
    };

    let _ = msg.reply(&ctx, &format!("The shard latency is {:?}", runner.latency));

    Ok(())
}

#[command]
// Limit command usage to guilds.
#[only_in(guilds)]
#[checks(Owner)]
fn ping(ctx: &mut Context, msg: &Message) -> CommandResult {
    if let Err(why) = msg.channel_id.say(&ctx.http, "Pong! : )") {
        println!("Error sending message: {:?}", why);
    }

    Ok(())
}

#[command]
// Adds multiple aliases
#[aliases("kitty", "neko")]
// Make this command use the "emoji" bucket.
#[bucket = "emoji"]
// Allow only administrators to call this:
#[required_permissions("ADMINISTRATOR")]
fn cat(ctx: &mut Context, msg: &Message) -> CommandResult {
    if let Err(why) = msg.channel_id.say(&ctx.http, ":cat:") {
        println!("Error sending message: {:?}", why);
    }

    Ok(())
}

#[command]
#[description = "Sends an emoji with a dog."]
#[bucket = "emoji"]
fn dog(ctx: &mut Context, msg: &Message) -> CommandResult {
    if let Err(why) = msg.channel_id.say(&ctx.http, ":dog:") {
        println!("Error sending message: {:?}", why);
    }

    Ok(())
}

#[command]
fn bird(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let say_content = if args.is_empty() {
        ":bird: can find animals for you.".to_string()
    } else {
        format!(":bird: could not find animal named: `{}`.", args.rest())
    };

    if let Err(why) = msg.channel_id.say(&ctx.http, say_content) {
        println!("Error sending message: {:?}", why);
    }

    Ok(())
}

#[command]
fn am_i_admin(ctx: &mut Context, msg: &Message) -> CommandResult {
    if let Err(why) = msg.channel_id.say(&ctx.http, "Yes you are.") {
        println!("Error sending message: {:?}", why);
    }

    Ok(())
}

#[command]
fn slow_mode(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let say_content = if let Ok(slow_mode_rate_seconds) = args.single::<u64>() {
        if let Err(why) = msg.channel_id.edit(&ctx.http, |c| c.slow_mode_rate(slow_mode_rate_seconds)) {
            println!("Error setting channel's slow mode rate: {:?}", why);

            format!("Failed to set slow mode to `{}` seconds.", slow_mode_rate_seconds)
        } else {
            format!("Successfully set slow mode rate to `{}` seconds.", slow_mode_rate_seconds)
        }
    } else if let Some(Channel::Guild(channel)) = msg.channel_id.to_channel_cached(&ctx.cache) {
        format!("Current slow mode rate is `{}` seconds.", channel.read().slow_mode_rate.unwrap_or(0))
    } else {
        "Failed to find channel in cache.".to_string()
    };

    if let Err(why) = msg.channel_id.say(&ctx.http, say_content) {
        println!("Error sending message: {:?}", why);
    }

    Ok(())
}
