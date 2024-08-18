use std::sync::Arc;
use serenity::all::{ChannelId, GuildId};
use songbird::{CoreEvent, Event};

use transmitter::Transmitter;

use crate::commands::receiver::Receiver;
use crate::commands::transmitter::TransmitterWrapper;
use crate::{Context, Error};

pub mod receiver;
pub mod transmitter;

pub fn retrieve_voice_channel(ctx: Context<'_>) -> Option<(GuildId, ChannelId)> {
    let guild = ctx.guild();
    guild.and_then(|guild| {
        if let Some(channel_id) = guild
            .voice_states
            .get(&ctx.author().id)
            .and_then(|voice_state| voice_state.channel_id) {
            Some((guild.id, channel_id))
        } else {
            None
        }
    })
}

#[poise::command(slash_command)]
pub async fn join(ctx: Context<'_>) -> Result<(), Error> {
    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();
    if let Some((guild_id, channel_id)) = retrieve_voice_channel(ctx) {
        if let Ok(call) = manager.join(guild_id, channel_id).await {
            let cloned_call = Arc::clone(&call);
            let mut locked_call = call.lock().await;
            {
                let receiver = ctx.data().receiver.clone();
                let mut locked_receiver = receiver.lock().await;
                *locked_receiver = Some(Receiver::new(cloned_call));
            }
            {
                let transmitter = ctx.data().transmitter.clone();
                let mut locked_transmitter = transmitter.lock().await;
                let new_transmitter = Arc::new(Transmitter::new());
                let new_transmitter_cloned = Arc::clone(&new_transmitter);
                let new_transmitter_wrapper = TransmitterWrapper::new(new_transmitter);
                locked_call.remove_all_global_events();
                locked_call.add_global_event(
                    Event::Core(CoreEvent::VoiceTick),
                    new_transmitter_wrapper,
                );
                *locked_transmitter = Some(new_transmitter_cloned);
            }
        }
    } else {
        /* User isn't connect to a voice channel */
    }
    Ok(())
}

#[poise::command(slash_command)]
pub async fn leave(ctx: Context<'_>) -> Result<(), Error> {
    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();
    if let Some(guild_id) = ctx.guild().map(|guild| guild.id) {
        if let Some(call) = manager.get(guild_id) {
            let mut locked_call = call.lock().await;
            locked_call.leave().await.expect("TODO: panic message");
            {
                let receiver = ctx.data().receiver.clone();
                let mut locked_receiver = receiver.lock().await;
                *locked_receiver = None;
            }
            {
                let transmitter = ctx.data().transmitter.clone();
                let mut locked_transmitter = transmitter.lock().await;
                locked_call.remove_all_global_events();
                *locked_transmitter = None;
            }
        }
    }
    Ok(())
}
