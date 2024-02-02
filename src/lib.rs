//! arma-3-status-bot

#![warn(missing_docs, rust_2018_idioms)]
#![cfg_attr(
    doc,
    warn(
        rustdoc::bare_urls,
        rustdoc::broken_intra_doc_links,
        rustdoc::invalid_codeblock_attributes,
        rustdoc::invalid_rust_codeblocks,
        rustdoc::missing_crate_level_docs
    )
)]

use std::{env, time::Duration};

use chrono::Utc;
use dotenv::dotenv;
use palette::{LinSrgb, Mix};
use poise::{
    serenity_prelude::{self as serenity, ActivityData, Colour, CreateEmbed, CreateEmbedFooter},
    CreateReply, Framework, FrameworkOptions,
};
use tokio::time;
use tracing::{debug, error, info, level_filters::LevelFilter, warn};
use tracing_subscriber::EnvFilter;

struct Data {}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

const ACTIVITY_UPDATE_INTERVAL: Duration = Duration::from_secs(30);

/// Entry point for the bot.
pub async fn run_bot() {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let token = match env::var("DISCORD_TOKEN") {
        Ok(token) => {
            info!(len = %token.len(), "successfully parsed token");
            token
        }
        Err(error) => {
            error!(%error, "failed to retrieve Discord token from env var");
            panic!("no Discord token");
        }
    };

    let intents = serenity::GatewayIntents::non_privileged();

    let framework = Framework::builder()
        .options(FrameworkOptions {
            commands: vec![register(), info()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                info!("running setup callback");

                match poise::builtins::register_globally(ctx, &framework.options().commands).await {
                    Ok(_) => info!("registered commands globally"),
                    Err(error) => error!(%error, "failed to register commands"),
                }

                tokio::spawn(activity_loop(ctx.clone()));

                Ok(Data {})
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    info!("running client...");
    client.unwrap().start().await.unwrap();
}

async fn activity_loop(ctx: serenity::Context) {
    let a2s_client = a2s::A2SClient::new().await.unwrap();

    loop {
        match a2s_client.info("100.124.122.71:2303").await {
            Ok(info) => {
                let activity = match info.players {
                    1 => ActivityData::playing(format!("Arma 3 with {} player", info.players)),
                    other => ActivityData::playing(format!("Arma 3 with {} players", other)),
                };
                ctx.set_activity(Some(activity));
                info!(
                    next_update = %humantime::Duration::from(ACTIVITY_UPDATE_INTERVAL),
                    "updated activity data"
                );
            }
            Err(error) => {
                warn!(
                    %error,
                    next_update = %humantime::Duration::from(ACTIVITY_UPDATE_INTERVAL),
                    "failed to retrieve arma data"
                );
            }
        }

        time::sleep(ACTIVITY_UPDATE_INTERVAL).await;
    }
}

#[poise::command(prefix_command)]
#[tracing::instrument(name = "register_command", skip_all)]
async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

#[poise::command(slash_command)]
#[tracing::instrument(name = "info_command", skip_all)]
async fn info(ctx: Context<'_>) -> Result<(), Error> {
    let client = a2s::A2SClient::new().await.unwrap();

    match tokio::time::timeout(
        Duration::from_millis(500),
        client.info("100.124.122.71:2303"),
    )
    .await
    {
        Ok(Ok(info)) => {
            debug!(?info, "got info");

            let max = info.max_players as f32;
            let players = info.players;

            let (empty_r, empty_g, empty_b) = Colour::DARK_PURPLE.tuple();
            let (full_r, full_g, full_b) = Colour::FABLED_PINK.tuple();

            let colour_result = LinSrgb::new(
                empty_r as f32 / 255.,
                empty_g as f32 / 255.,
                empty_b as f32 / 255.,
            )
            .mix(
                LinSrgb::new(
                    full_r as f32 / 255.,
                    full_g as f32 / 255.,
                    full_b as f32 / 255.,
                ),
                players as f32 / max,
            );

            let final_colour = Colour::from_rgb(
                (colour_result.red * 255.) as u8,
                (colour_result.green * 255.) as u8,
                (colour_result.blue * 255.) as u8,
            );

            let mut embed = CreateEmbed::new()
                .title("ArmA 3 Server Info")
                .description(
                    "The ArmA 3 server is running; details for how to join are in the \
                     <#1196289531842928791> channel.",
                )
                .colour(final_colour)
                .field("game", info.game.to_string(), false)
                .field("players", players.to_string(), true)
                .field("max players", info.max_players.to_string(), true)
                .timestamp(Utc::now());

            if let Ok(version) = env::var("CARGO_PKG_VERSION") {
                embed = embed.footer(CreateEmbedFooter::new(format!(
                    "arma-3-status-bot {}",
                    version
                )));
            }

            if let Ok(url) = env::var("GITHUB_REPO_URL") {
                embed = embed.url(url);
            }

            ctx.send(CreateReply::default().embed(embed)).await?;
        }
        Ok(Err(error)) => {
            warn!(%error, "failed to retrieve arma data");

            let mut embed = CreateEmbed::new()
                .title("ArmA 3 Server Info")
                .description(
                    "The ArmA 3 server doesn't appear to be running right now. Please bear with!",
                )
                .colour(Colour::RED)
                .timestamp(Utc::now());

            if let Ok(version) = env::var("CARGO_PKG_VERSION") {
                embed = embed.footer(CreateEmbedFooter::new(format!(
                    "arma-3-status-bot {}",
                    version
                )));
            }

            ctx.send(CreateReply::default().embed(embed)).await?;
        }
        Err(error) => {
            warn!(%error, "timed out while retrieving arma data");

            let mut embed = CreateEmbed::new()
                .title("ArmA 3 Server Info")
                .description(
                    "The ArmA 3 server doesn't appear to be running right now. Please bear with!",
                )
                .colour(Colour::RED)
                .timestamp(Utc::now());

            if let Ok(version) = env::var("CARGO_PKG_VERSION") {
                embed = embed.footer(CreateEmbedFooter::new(format!(
                    "arma-3-status-bot {}",
                    version
                )));
            }

            ctx.send(CreateReply::default().embed(embed)).await?;
        }
    }

    Ok(())
}
