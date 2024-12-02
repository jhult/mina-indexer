use crate::{
    constants::POSTGRES_CONNECTION_STRING,
    event_sourcing::{db_logger::DbLogger, events::Event, payloads::*},
};
use futures::lock::Mutex;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::Arc;
use tokio_postgres::NoTls;

pub struct CanonicalInternalCommandLogPersistenceActor {
    db_logger: Arc<Mutex<DbLogger>>,
}

impl Default for CanonicalInternalCommandLogPersistenceActor {
    fn default() -> Self {
        todo!()
    }
}

#[async_trait::async_trait]
impl Actor for CanonicalInternalCommandLogPersistenceActor {
    type Msg = Event;
    type State = ActorRef<Event>;
    type Arguments = Option<(u64, String)>;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, args: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .map_err(|_| ActorProcessingErr::from("Unable to connect to database"))?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        let logger = DbLogger::builder(client)
            .name("internal_commands")
            .add_column("internal_command_type TEXT NOT NULL")
            .add_column("height BIGINT NOT NULL")
            .add_column("state_hash TEXT NOT NULL")
            .add_column("timestamp BIGINT NOT NULL")
            .add_column("amount_nanomina BIGINT NOT NULL")
            .add_column("recipient TEXT NOT NULL")
            .add_column("is_canonical BOOLEAN NOT NULL")
            .distinct_columns(&["height", "internal_command_type", "state_hash", "recipient", "amount_nanomina"])
            .build(&args)
            .await
            .map_err(|_| ActorProcessingErr::from("Failed to build internal_commands_log and internal_commands view"))?;

        Ok(Self {
            db_logger: Arc::new(Mutex::new(logger)),
        })
    }

    async fn handle(&self, ctx: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match msg {
            Event::CanonicalInternalCommandLog(payload) => {
                let logger = self.db_logger.lock().await;
                let affected_rows = logger
                    .insert(
                        &[
                            &payload.internal_command_type.to_string(),
                            &(payload.height as i64),
                            &payload.state_hash,
                            &(payload.timestamp as i64),
                            &(payload.amount_nanomina as i64),
                            &payload.recipient,
                            &payload.canonical,
                        ],
                        payload.height,
                    )
                    .await
                    .map_err(|_| ActorProcessingErr::from("Unable to insert into canonical_internal_commands_log table"))?;

                if affected_rows != 1 {
                    return Err(ActorProcessingErr::from("Expected exactly one row to be affected"));
                }

                let actor_height = ActorHeightPayload {
                    actor: "CanonicalInternalCommandLogPersistenceActor".to_string(),
                    height: payload.height,
                };
                state.cast(Event::ActorHeight(actor_height))?;
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod canonical_internal_command_log_tests {
    use super::*;
    use crate::event_sourcing::{payloads::InternalCommandType, test_utils::setup_test_actors};
    use ractor::rpc::CallResult;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_insert_canonical_internal_command_log() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(CanonicalInternalCommandLogPersistenceActor::default()).await;

        let payload = CanonicalInternalCommandLogPayload {
            internal_command_type: InternalCommandType::FeeTransfer,
            height: 100,
            state_hash: "state_hash_100".to_string(),
            timestamp: 1234567890,
            amount_nanomina: 500,
            recipient: "recipient_1".to_string(),
            canonical: true,
            source: None,
            was_canonical: false,
        };

        let response = actor
            .call(|_| Event::CanonicalInternalCommandLog(payload.clone()), Some(Duration::from_secs(1)))
            .await?;

        // Validate that exactly one row was affected
        match response {
            CallResult::Success(Event::ActorHeight(actor_height_payload)) => {
                assert_eq!(actor_height_payload.height, payload.height);
                assert_eq!(actor_height_payload.actor, "CanonicalInternalCommandLogPersistenceActor");
            }
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_handle_event_canonical_internal_command_log() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(CanonicalInternalCommandLogPersistenceActor::default()).await;

        let payload = CanonicalInternalCommandLogPayload {
            internal_command_type: InternalCommandType::Coinbase,
            height: 200,
            state_hash: "state_hash_200".to_string(),
            timestamp: 987654321,
            amount_nanomina: 1000,
            recipient: "recipient_2".to_string(),
            canonical: false,
            was_canonical: false,
            source: None,
        };

        let response = actor
            .call(|_| Event::CanonicalInternalCommandLog(payload.clone()), Some(Duration::from_secs(1)))
            .await?;

        // Validate the response
        match response {
            CallResult::Success(Event::ActorHeight(actor_height_payload)) => {
                assert_eq!(actor_height_payload.height, payload.height);
                assert_eq!(actor_height_payload.actor, "CanonicalInternalCommandLogPersistenceActor");
            }
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_canonical_internal_commands_view() -> anyhow::Result<()> {
        use crate::event_sourcing::{payloads::InternalCommandType, test_utils::setup_test_actors};
        use tokio_postgres::NoTls;

        // Setup test actor
        let actor = setup_test_actors(CanonicalInternalCommandLogPersistenceActor::default()).await;

        // Insert multiple entries for the same (height, state_hash, recipient, amount) with different timestamps and canonicalities
        let payload1 = CanonicalInternalCommandLogPayload {
            internal_command_type: InternalCommandType::FeeTransfer,
            height: 1,
            state_hash: "hash_1".to_string(),
            timestamp: 1234567890,
            amount_nanomina: 1000,
            recipient: "recipient_1".to_string(),
            canonical: false,
            was_canonical: false,
            source: None,
        };

        let payload2 = CanonicalInternalCommandLogPayload {
            internal_command_type: InternalCommandType::FeeTransfer,
            height: 1,
            state_hash: "hash_1".to_string(),
            timestamp: 1234567891, // Later timestamp
            amount_nanomina: 1000,
            recipient: "recipient_1".to_string(),
            canonical: true,
            was_canonical: false,
            source: None,
        };

        actor
            .handle(
                ActorRef::default(),
                Event::CanonicalInternalCommandLog(payload1.clone()),
                &mut ActorRef::default(),
            )
            .await
            .unwrap();
        actor
            .handle(
                ActorRef::default(),
                Event::CanonicalInternalCommandLog(payload2.clone()),
                &mut ActorRef::default(),
            )
            .await
            .unwrap();

        // Query the internal_commands view
        let query = "SELECT * FROM internal_commands WHERE height = $1 AND state_hash = $2 AND recipient = $3";
        let logger = actor.db_logger.lock().await;
        let row = logger
            .get_client()
            .query_one(query, &[&(payload1.height as i64), &payload1.state_hash, &payload1.recipient])
            .await
            .unwrap();

        // Validate that the row returned matches the payload with the highest timestamp
        assert_eq!(row.get::<_, String>("internal_command_type"), payload2.internal_command_type.to_string());
        assert_eq!(row.get::<_, i64>("height"), payload2.height as i64);
        assert_eq!(row.get::<_, String>("state_hash"), payload2.state_hash);
        assert_eq!(row.get::<_, i64>("timestamp"), payload2.timestamp as i64);
        assert_eq!(row.get::<_, i64>("amount_nanomina"), payload2.amount_nanomina as i64);
        assert_eq!(row.get::<_, String>("recipient"), payload2.recipient);
        assert_eq!(row.get::<_, bool>("is_canonical"), payload2.canonical);

        Ok(())
    }

    #[tokio::test]
    async fn test_actor_height_event_published() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(CanonicalInternalCommandLogPersistenceActor::default()).await;

        // Create a payload for a canonical internal command
        let payload = CanonicalInternalCommandLogPayload {
            internal_command_type: InternalCommandType::Coinbase,
            height: 150,
            state_hash: "state_hash_150".to_string(),
            timestamp: 1234567890,
            amount_nanomina: 10000,
            recipient: "recipient_150".to_string(),
            canonical: true,
            source: None,
            was_canonical: false,
        };

        let response = actor
            .call(|_| Event::CanonicalInternalCommandLog(payload.clone()), Some(Duration::from_secs(1)))
            .await?;

        // Verify the event type is `ActorHeight`
        match response {
            CallResult::Success(Event::ActorHeight(actor_height_payload)) => {
                assert_eq!(actor_height_payload.height, payload.height);
                assert_eq!(actor_height_payload.actor, "CanonicalInternalCommandLogPersistenceActor");
            }
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        Ok(())
    }
}
