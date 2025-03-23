use crate::utils::{BotConfig, BotConn, InvokePayload};
use anyhow::bail;
use clap::Parser;
use log::{debug, error, info};
use prost::Message;
use serde_json::json;
use std::time::Duration;
use valveprotos::deadlock::c_msg_client_to_gc_get_match_meta_data_response::EResult;
use valveprotos::deadlock::{
    CMsgClientToGcGetMatchMetaData, CMsgClientToGcGetMatchMetaDataResponse,
    EgcCitadelClientMessages,
};

mod utils;

// create a clap struct where user can input username password and match id
#[derive(Debug, Clone, Parser)]
pub struct Args {
    #[clap(short, long, env)]
    pub username: String,
    #[clap(short, long, env)]
    pub password: String,
    #[clap(short, long, value_parser, num_args = 1.., value_delimiter = ' ')]
    pub match_ids: Vec<u64>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();

    let config = BotConfig {
        username: args.username.clone(),
        password: args.password.clone(),
    };
    let bot = utils::create_bot(&config).await;
    let bot = match bot {
        Ok(bot) => {
            info!("bot created: {:?}", bot);
            bot
        }
        Err(e) => {
            bail!("error creating bot: {:?}", e);
        }
    };

    for match_id in args.match_ids {
        let result = tryhard::retry_fn(|| fetch_match(match_id, &bot))
            .retries(3)
            .exponential_backoff(Duration::from_millis(100))
            .await;
        match result {
            Ok(_) => info!("Match Salts fetched"),
            Err(e) => error!("Error fetching match salts: {:?}", e),
        }
    }

    bot.disconnect().await?;

    Ok(())
}

async fn fetch_match(match_id: u64, bot: &BotConn) -> anyhow::Result<()> {
    let msg = CMsgClientToGcGetMatchMetaData {
        match_id: match_id.into(),
        ..Default::default()
    };
    let serialized_message = msg.encode_to_vec();
    let payload = InvokePayload {
        kind: EgcCitadelClientMessages::KEMsgClientToGcGetMatchMetaData.into(),
        data: serialized_message,
    };
    let result = bot.invoke_with_retries(&payload, 3).await?.data;
    let msg = CMsgClientToGcGetMatchMetaDataResponse::decode(&result[..])?;

    let result = msg
        .result
        .and_then(|r| EResult::try_from(r).ok())
        .unwrap_or(EResult::KEResultInternalError);

    match result {
        EResult::KEResultSuccess => {
            debug!("Got Match Salts: {:?}", msg);
            reqwest::Client::new()
                .post("https://api.deadlock-api.com/v1/matches/salts")
                .json(&[json!({
                    "cluster_id": msg.replay_group_id,
                    "match_id": match_id,
                    "metadata_salt": msg.metadata_salt,
                    "replay_salt": msg.replay_salt,
                    "username": bot.bot_name
                })])
                .send()
                .await?
                .error_for_status()?;
            debug!("Ingested Salts");
        }
        EResult::KEResultRateLimited => {
            bail!("Rate Limited");
        }
        _ => {
            bail!("Error: {:?}", msg.result);
        }
    }
    Ok(())
}
