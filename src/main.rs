use std::env;
use std::sync::Arc;

use dotenv::dotenv;
use serenity::all::ClientBuilder;
use serenity::prelude::*;
use songbird::{Config, driver::DecodeMode, SerenityInit};

use crate::commands::receiver::Receiver;
use crate::commands::transmitter::Transmitter;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;
pub struct Data {
    receiver: Arc<Mutex<Option<Receiver>>>,
    transmitter: Arc<Mutex<Option<Arc<Transmitter>>>>
}

mod commands;

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();
    env::var("TARGET_RX_ADDR").expect("Expected a target rx address in the environment");
    env::var("LOCAL_RX_ADDR").expect("Expected a local rx address in the environment");
    let token = env::var("BOT_TOKEN").expect("Expected a token in the environment");


    // FrameworkOptions contains all of poise's configuration option in one struct
    // Every option can be omitted to use its default value
    let options = poise::FrameworkOptions {
        commands: vec![commands::join(), commands::leave()],
        ..Default::default()
    };

    let songbird_config = Config::default().decode_mode(DecodeMode::Decode);

    let framework = poise::Framework::builder()
        .setup(move |ctx, ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    receiver: Arc::new(Mutex::new(None)),
                    transmitter: Arc::new(Mutex::new(None)),
                })
            })
        })
        .options(options)
        .build();

    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES;

    let client = ClientBuilder::new(token, intents)
        .framework(framework)
        .register_songbird_from_config(songbird_config)
        .await;

    client.unwrap().start().await.unwrap()
}
