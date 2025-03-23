use anyhow::{Context, bail};
use futures::future::select;
use log::{debug, info, warn};
use prost::Message;
use std::pin::pin;
use std::time::Duration;
use steam_vent::proto::enums_clientserver::EMsg;
use steam_vent::proto::steammessages_clientserver::CMsgClientGamesPlayed;
use steam_vent::proto::steammessages_clientserver::cmsg_client_games_played::GamePlayed;
use steam_vent::{
    Connection, ConnectionTrait, GameCoordinator, RawNetMessage, UntypedMessage,
    proto::{MsgKind, steammessages_clientserver_login::CMsgClientLoggedOff},
};
use steam_vent::{
    NetworkError, ServerList,
    auth::{
        AuthConfirmationHandler as _, ConsoleAuthConfirmationHandler, DeviceConfirmationHandler,
        FileGuardDataStore,
    },
    proto::steammessages_clientserver_login::CMsgClientLogOff,
};
use tokio::time::{sleep, timeout};
use valveprotos::deadlock::{CMsgCitadelClientHello, EgcCitadelClientMessages};
use valveprotos::gcsdk::CMsgConnectionStatus;

#[derive(Debug, Default, Clone)]
pub struct BotConfig {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct InvokePayload {
    pub kind: i32,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct BotConn {
    conn: Connection,
    gc: GameCoordinator,
    pub bot_name: String,
}

impl BotConn {
    pub async fn invoke(&self, payload: InvokePayload) -> Result<RawNetMessage, NetworkError> {
        debug!("Invoking job of kind: {}", payload.kind);
        let msg = UntypedMessage(payload.data);

        self.gc.job_untyped(msg, MsgKind(payload.kind), true).await
    }

    pub async fn invoke_with_retries(
        &self,
        payload: &InvokePayload,
        max_retries: i32,
    ) -> Result<RawNetMessage, NetworkError> {
        let mut retries = 0;

        loop {
            match self.invoke(payload.clone()).await {
                Ok(v) => return Ok(v),
                Err(e) => {
                    retries += 1;

                    if retries > max_retries {
                        debug!("Failing due to exceeding max retries");
                        return Err(e);
                    }

                    let sleep_time_s = 2 * retries;
                    let sleep_dur = Duration::from_secs(sleep_time_s as u64);
                    debug!(
                        "Retrying after got error during invoke: {:?}. Attempt #{}. Sleeping for {}s",
                        e, retries, sleep_time_s
                    );

                    sleep(sleep_dur).await;
                }
            }
        }
    }

    /// Logoff
    pub async fn disconnect(&self) -> anyhow::Result<()> {
        let logoff_msg = CMsgClientLogOff {
            ..Default::default()
        };
        self.conn.send(logoff_msg).await?;
        match tokio::time::timeout(
            Duration::from_secs(10),
            self.conn.one::<CMsgClientLoggedOff>(),
        )
        .await
        {
            Ok(Ok(v)) => {
                debug!("Got CMsgClientLoggedOff result: {}", v.eresult());
            }
            Ok(Err(e)) => {
                bail!("Error logging off: {:?}", e);
            }
            Err(_) => {
                bail!("Never received LoggedOff response");
            }
        };

        info!("Successfully logged off");

        Ok(())
    }
}

pub async fn create_bot(cfg: &BotConfig) -> anyhow::Result<BotConn> {
    let server_list = ServerList::discover().await?;
    let mut connection = Connection::login(
        &server_list,
        &cfg.username,
        &cfg.password,
        FileGuardDataStore::user_cache(),
        ConsoleAuthConfirmationHandler::default().or(DeviceConfirmationHandler),
    )
    .await?;

    connection.set_timeout(Duration::from_secs(20));

    let game_coordinator = GameCoordinator::new_without_startup(&connection, 1422450).await?;

    timeout(
        Duration::from_secs(60),
        deadlock_startup_seq(&connection, &game_coordinator),
    )
    .await
    .context("failed startup seq")??;

    let bc = BotConn {
        conn: connection,
        gc: game_coordinator,
        bot_name: cfg.username.clone(),
    };

    debug!("Bot connected!");
    Ok(bc)
}

pub async fn deadlock_startup_seq(
    connection: &Connection,
    gc: &GameCoordinator,
) -> anyhow::Result<()> {
    connection
        .send_with_kind(
            CMsgClientGamesPlayed {
                games_played: vec![GamePlayed {
                    game_id: Some(1422450_u64),
                    ..Default::default()
                }],
                ..Default::default()
            },
            EMsg::k_EMsgClientGamesPlayedWithDataBlob,
        )
        .await?;

    sleep(Duration::from_secs(2)).await;

    let welcome_playtest = async {
        match gc
            .filter
            .one_kind(MsgKind(
                EgcCitadelClientMessages::KEMsgGcToClientDevPlaytestStatus as i32,
            ))
            .await
        {
            Ok(_) => {
                debug!("Got playtest");
                Ok(())
            }
            Err(e) => Err(e),
        }
    };
    let mut connection_status_subscriber = gc.filter.on_kind(MsgKind(4009));
    tokio::spawn(async move {
        while let Ok(msg) = connection_status_subscriber.recv().await {
            let data = msg.data;
            let Ok(msg) = CMsgConnectionStatus::decode(data) else {
                warn!("Got invalid game server message");
                continue;
            };
            debug!("Got connection status: {:?}", msg);
        }
    });
    let hello_sender = async {
        loop {
            let hello_msg = CMsgCitadelClientHello {};

            let encoded = UntypedMessage(hello_msg.encode_to_vec());

            debug!("Sending hello");
            if let Err(e) = gc.send_untyped(encoded, MsgKind(4006), true).await {
                return Result::<(), _>::Err(e);
            };
            sleep(Duration::from_secs(5)).await;
        }
    };

    select(pin!(welcome_playtest), pin!(hello_sender)).await;

    Ok(())
}
