use crate::{
    constants::POSTGRES_CONNECTION_STRING,
    stream::{db_logger::DbLogger, events::Event, payloads::ActorHeightPayload},
};
use futures::lock::Mutex;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::Arc;
use tokio_postgres::NoTls;

pub struct CanonicalBlockLogPersistenceActor {
    db_logger: Arc<Mutex<DbLogger>>,
}

impl CanonicalBlockLogPersistenceActor {
    pub fn new() -> Self {
        Self {
            db_logger: Arc::new(Mutex::new(DbLogger::default())),
        }
    }
}

#[async_trait::async_trait]
impl Actor for CanonicalBlockLogPersistenceActor {
    type Msg = Event;
    type State = ActorRef<Event>;
    type Arguments = ActorRef<Event>;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, parent: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .map_err(|_| ActorProcessingErr::from("Unable to establish connection to database"))?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        let logger = DbLogger::builder(client)
            .name("blocks")
            .add_column("height BIGINT")
            .add_column("state_hash TEXT")
            .add_column("previous_state_hash TEXT")
            .add_column("user_command_count INTEGER")
            .add_column("snark_work_count INTEGER")
            .add_column("timestamp BIGINT")
            .add_column("coinbase_receiver TEXT")
            .add_column("coinbase_reward_nanomina BIGINT")
            .add_column("global_slot_since_genesis BIGINT")
            .add_column("last_vrf_output TEXT")
            .add_column("is_berkeley_block BOOLEAN")
            .add_column("canonical BOOLEAN")
            .distinct_columns(&["height", "state_hash"])
            .build(&None)
            .await
            .map_err(|_| ActorProcessingErr::from("Failed to build blocks_log and blocks view"))?;

        *self.db_logger.lock().await = logger;
        Ok(parent)
    }

    async fn handle(&self, _myself: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        if let Event::CanonicalBlockLog(log) = msg {
            let logger = self.db_logger.lock().await;
            logger
                .insert(&[
                    &(log.height as i64),
                    &log.state_hash,
                    &log.previous_state_hash,
                    &(log.user_command_count as i32),
                    &(log.snark_work_count as i32),
                    &(log.timestamp as i64),
                    &log.coinbase_receiver,
                    &(log.coinbase_reward_nanomina as i64),
                    &(log.global_slot_since_genesis as i64),
                    &log.last_vrf_output,
                    &log.is_berkeley_block,
                    &log.canonical,
                ])
                .await
                .map_err(|_| ActorProcessingErr::from("Unable to insert into canonical_block_log table"))?;

            let actor_height = ActorHeightPayload {
                actor: "CanonicalBlockLogPersistenceActor".to_string(),
                height: log.height,
            };
            state.cast(Event::ActorHeight(actor_height))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::{
        payloads::{ActorHeightPayload, CanonicalBlockLogPayload},
        test_utils::setup_test_actors,
    };
    use ractor::rpc::CallResult;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_insert_canonical_block_log() -> anyhow::Result<()> {
        let actor = setup_test_actors(CanonicalBlockLogPersistenceActor::new()).await;

        let payload = CanonicalBlockLogPayload {
            height: 100,
            state_hash: "state_hash_100".to_string(),
            previous_state_hash: "state_hash_99".to_string(),
            user_command_count: 5,
            snark_work_count: 3,
            timestamp: 1234567890,
            coinbase_receiver: "coinbase_receiver".to_string(),
            coinbase_reward_nanomina: 1000,
            global_slot_since_genesis: 50,
            last_vrf_output: "vrf_output".to_string(),
            is_berkeley_block: true,
            canonical: true,
        };

        let response = actor.call(|_| Event::CanonicalBlockLog(payload.clone()), Some(Duration::from_secs(1))).await?;

        match response {
            CallResult::Success(Event::ActorHeight(height_payload)) => {
                assert_eq!(height_payload.height, payload.height);
                assert_eq!(height_payload.actor, "CanonicalBlockLogPersistenceActor");
            }
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        let db_logger = actor.db_logger.lock().await;
        let query = "SELECT * FROM blocks_log WHERE height = $1 AND state_hash = $2 AND timestamp = $3";
        let row = db_logger
            .get_client()
            .query_one(query, &[&(payload.height as i64), &payload.state_hash, &(payload.timestamp as i64)])
            .await?;

        assert_eq!(row.get::<_, i64>("height"), payload.height as i64);
        assert_eq!(row.get::<_, String>("state_hash"), payload.state_hash);
        assert_eq!(row.get::<_, String>("previous_state_hash"), payload.previous_state_hash);
        assert_eq!(row.get::<_, i32>("user_command_count"), payload.user_command_count as i32);
        assert_eq!(row.get::<_, i32>("snark_work_count"), payload.snark_work_count as i32);
        assert_eq!(row.get::<_, i64>("timestamp"), payload.timestamp as i64);
        assert_eq!(row.get::<_, String>("coinbase_receiver"), payload.coinbase_receiver);
        assert_eq!(row.get::<_, i64>("coinbase_reward_nanomina"), payload.coinbase_reward_nanomina as i64);
        assert_eq!(row.get::<_, i64>("global_slot_since_genesis"), payload.global_slot_since_genesis as i64);
        assert_eq!(row.get::<_, String>("last_vrf_output"), payload.last_vrf_output);
        assert_eq!(row.get::<_, bool>("is_berkeley_block"), payload.is_berkeley_block);
        assert_eq!(row.get::<_, bool>("canonical"), payload.canonical);

        Ok(())
    }

    #[tokio::test]
    async fn test_canonical_blocks_canonical_block_log() -> anyhow::Result<()> {
        let actor = setup_test_actors(CanonicalBlockLogPersistenceActor::new()).await;

        let payload1 = CanonicalBlockLogPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            previous_state_hash: "prev_hash".to_string(),
            user_command_count: 10,
            snark_work_count: 2,
            timestamp: 1234567890,
            coinbase_receiver: "receiver_1".to_string(),
            coinbase_reward_nanomina: 1000,
            global_slot_since_genesis: 100,
            last_vrf_output: "vrf_output_1".to_string(),
            is_berkeley_block: false,
            canonical: false,
        };

        let mut payload2 = payload1.clone();
        payload2.canonical = true;

        let response = actor.call(|_| Event::CanonicalBlockLog(payload1), Some(Duration::from_secs(1))).await?;

        match response {
            CallResult::Success(Event::ActorHeight(_)) => {}
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        let response = actor.call(|_| Event::CanonicalBlockLog(payload2.clone()), Some(Duration::from_secs(1))).await?;

        match response {
            CallResult::Success(Event::ActorHeight(_)) => {}
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        let db_logger = actor.db_logger.lock().await;
        let query = "SELECT * FROM blocks WHERE height = $1";
        let rows = db_logger.get_client().query(query, &[&1_i64]).await?;
        assert_eq!(rows.len(), 1);

        let row = rows.first().unwrap();
        assert_eq!(row.get::<_, i64>("height"), payload2.height as i64);
        assert_eq!(row.get::<_, String>("state_hash"), payload2.state_hash);
        assert_eq!(row.get::<_, String>("previous_state_hash"), payload2.previous_state_hash);
        assert_eq!(row.get::<_, i32>("user_command_count"), payload2.user_command_count as i32);
        assert_eq!(row.get::<_, i32>("snark_work_count"), payload2.snark_work_count as i32);
        assert_eq!(row.get::<_, i64>("timestamp"), payload2.timestamp as i64);
        assert_eq!(row.get::<_, String>("coinbase_receiver"), payload2.coinbase_receiver);
        assert_eq!(row.get::<_, i64>("coinbase_reward_nanomina"), payload2.coinbase_reward_nanomina as i64);
        assert_eq!(row.get::<_, i64>("global_slot_since_genesis"), payload2.global_slot_since_genesis as i64);
        assert_eq!(row.get::<_, String>("last_vrf_output"), payload2.last_vrf_output);
        assert_eq!(row.get::<_, bool>("is_berkeley_block"), payload2.is_berkeley_block);
        assert_eq!(row.get::<_, bool>("canonical"), payload2.canonical);

        Ok(())
    }

    #[tokio::test]
    async fn test_canonical_blocks_view() -> anyhow::Result<()> {
        let actor = setup_test_actors(CanonicalBlockLogPersistenceActor::new()).await;

        let payload1 = CanonicalBlockLogPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            previous_state_hash: "prev_hash".to_string(),
            user_command_count: 10,
            snark_work_count: 2,
            timestamp: 1234567890,
            coinbase_receiver: "receiver_1".to_string(),
            coinbase_reward_nanomina: 1000,
            global_slot_since_genesis: 100,
            last_vrf_output: "vrf_output_1".to_string(),
            is_berkeley_block: false,
            canonical: false,
        };

        let mut payload2 = payload1.clone();
        payload2.canonical = true;

        let response = actor.call(|_| Event::CanonicalBlockLog(payload1), Some(Duration::from_secs(1))).await?;

        match response {
            CallResult::Success(Event::ActorHeight(_)) => {}
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        let response = actor.call(|_| Event::CanonicalBlockLog(payload2.clone()), Some(Duration::from_secs(1))).await?;

        match response {
            CallResult::Success(Event::ActorHeight(_)) => {}
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        let db_logger = actor.db_logger.lock().await;
        let query = "SELECT * FROM blocks WHERE height = $1";
        let row = db_logger.get_client().query_one(query, &[&1_i64]).await?;

        assert_eq!(row.get::<_, i64>("height"), payload2.height as i64);
        assert_eq!(row.get::<_, String>("state_hash"), payload2.state_hash);
        assert_eq!(row.get::<_, String>("previous_state_hash"), payload2.previous_state_hash);
        assert_eq!(row.get::<_, i32>("user_command_count"), payload2.user_command_count as i32);
        assert_eq!(row.get::<_, i32>("snark_work_count"), payload2.snark_work_count as i32);
        assert_eq!(row.get::<_, i64>("timestamp"), payload2.timestamp as i64);
        assert_eq!(row.get::<_, String>("coinbase_receiver"), payload2.coinbase_receiver);
        assert_eq!(row.get::<_, i64>("coinbase_reward_nanomina"), payload2.coinbase_reward_nanomina as i64);
        assert_eq!(row.get::<_, i64>("global_slot_since_genesis"), payload2.global_slot_since_genesis as i64);
        assert_eq!(row.get::<_, String>("last_vrf_output"), payload2.last_vrf_output);
        assert_eq!(row.get::<_, bool>("is_berkeley_block"), payload2.is_berkeley_block);
        assert_eq!(row.get::<_, bool>("canonical"), payload2.canonical);

        Ok(())
    }

    #[tokio::test]
    async fn test_actor_height_event_published() -> anyhow::Result<()> {
        let actor = setup_test_actors(CanonicalBlockLogPersistenceActor::new()).await;

        let payload = CanonicalBlockLogPayload {
            height: 300,
            state_hash: "state_hash_300".to_string(),
            previous_state_hash: "state_hash_299".to_string(),
            user_command_count: 15,
            snark_work_count: 5,
            timestamp: 1627891234,
            coinbase_receiver: "receiver_300".to_string(),
            coinbase_reward_nanomina: 3000,
            global_slot_since_genesis: 120,
            last_vrf_output: "vrf_output_300".to_string(),
            is_berkeley_block: false,
            canonical: true,
        };

        let response = actor.call(|_| Event::CanonicalBlockLog(payload), Some(Duration::from_secs(1))).await?;

        match response {
            CallResult::Success(Event::ActorHeight(height_payload)) => {
                assert_eq!(height_payload.actor, "CanonicalBlockLogPersistenceActor");
                assert_eq!(height_payload.height, 300);
            }
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        Ok(())
    }
}
