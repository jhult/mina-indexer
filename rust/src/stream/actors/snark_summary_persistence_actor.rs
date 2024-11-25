use crate::{
    constants::POSTGRES_CONNECTION_STRING,
    stream::{events::Event, payloads::SnarkCanonicitySummaryPayload},
};
use anyhow::Result;
use async_trait::async_trait;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use tokio_postgres::{Client, NoTls};

pub struct SnarkSummaryPersistenceActor {
    pub client: Option<Client>,
}

impl SnarkSummaryPersistenceActor {
    async fn db_upsert(&self, summary: &SnarkCanonicitySummaryPayload) -> Result<u64, &'static str> {
        let client = self.client.as_ref().expect("Client should be initialized");
        let upsert_query = r#"
            INSERT INTO snark_work_summary (
                height,
                state_hash,
                timestamp,
                prover,
                fee,
                is_canonical
            ) VALUES
                ($1, $2, $3, $4, $5, $6)
            ON CONFLICT ON CONSTRAINT unique_snark_work_summary
            DO UPDATE SET
                state_hash = EXCLUDED.state_hash,
                timestamp = EXCLUDED.timestamp,
                prover = EXCLUDED.prover,
                fee = EXCLUDED.fee,
                is_canonical = EXCLUDED.is_canonical;
            "#;

        match client
            .execute(
                upsert_query,
                &[
                    &(summary.height as i64),
                    &summary.state_hash,
                    &(summary.timestamp as i64),
                    &summary.prover,
                    &{ summary.fee },
                    &summary.canonical,
                ],
            )
            .await
        {
            Err(e) => {
                let msg = e.to_string();
                println!("{}", msg);
                Err("unable to upsert into snark_work_summary table")
            }
            Ok(affected_rows) => Ok(affected_rows),
        }
    }
}

#[async_trait::async_trait]
impl Actor for SnarkSummaryPersistenceActor {
    type Msg = Event;
    type State = ActorRef<Event>;
    type Arguments = ActorRef<Event>;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, parent: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        // Initialize database connection
        let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .map_err(|e| ActorProcessingErr::Fatal(Box::new(e)))?;

        // Spawn connection handler
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        // Create table if it doesn't exist
        client
            .execute(
                "CREATE TABLE IF NOT EXISTS snark_work_summary (
                    height BIGINT NOT NULL,
                    state_hash TEXT NOT NULL,
                    timestamp BIGINT NOT NULL,
                    prover TEXT NOT NULL,
                    fee DOUBLE PRECISION NOT NULL,
                    is_canonical BOOLEAN NOT NULL,
                    CONSTRAINT unique_snark_work_summary UNIQUE (height, state_hash, timestamp, prover, fee)
                );",
                &[],
            )
            .await
            .map_err(|e| ActorProcessingErr::Fatal(Box::new(e)))?;

        Ok(Self { client: Some(client) })
    }

    async fn handle(&self, ctx: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match msg {
            Event::SnarkCanonicitySummary(payload) => match state.db_upsert(&payload).await {
                Ok(affected_rows) => {
                    assert_eq!(affected_rows, 1);
                    ctx.send_parent(Event::ActorHeight(ActorHeightPayload {
                        height: payload.height,
                        actor: "SnarkSummaryPersistenceActor".to_string(),
                    }))?;
                }
                Err(e) => {
                    return Err(ActorProcessingErr::Fatal(e.into()));
                }
            },
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod snark_summary_persistence_actor_tests {
    use super::*;
    use crate::stream::payloads::{ActorHeightPayload, SnarkCanonicitySummaryPayload};
    use std::sync::Arc;
    use tokio::time::timeout;

    async fn setup_actor() -> (SnarkSummaryPersistenceActor, tokio::sync::broadcast::Receiver<Event>) {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = SnarkSummaryPersistenceActor::new(Arc::clone(&shared_publisher), &None).await;
        let receiver = shared_publisher.subscribe();
        (actor, receiver)
    }

    #[tokio::test]
    async fn test_snark_summary_persistence_actor_logs_summary() {
        let (actor, mut receiver) = setup_actor().await;

        let snark_summary = SnarkCanonicitySummaryPayload {
            height: 10,
            state_hash: "test_hash".to_string(),
            timestamp: 123456,
            prover: "test_prover".to_string(),
            fee_nanomina: 250000000,
            canonical: true,
        };

        let event = Event {
            event_type: Event::SnarkCanonicitySummary,
            payload: sonic_rs::to_string(&snark_summary).unwrap(),
        };

        // Handle the event
        actor.handle_event(event).await;

        // Verify the ActorHeight event is published
        if let Ok(event) = timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
            let published_event: ActorHeightPayload = sonic_rs::from_str(&event.unwrap().payload).unwrap();
            assert_eq!(published_event.actor, actor.id());
            assert_eq!(published_event.height, snark_summary.height);
        } else {
            panic!("Expected ActorHeight event was not published.");
        }
    }

    #[tokio::test]
    async fn test_snark_summary_persistence_actor_logs_to_database() {
        let (actor, _) = setup_actor().await;

        let snark_summary = SnarkCanonicitySummaryPayload {
            height: 15,
            state_hash: "test_hash_2".to_string(),
            timestamp: 789012,
            prover: "test_prover_2".to_string(),
            fee_nanomina: 500000000,
            canonical: false,
        };

        // Log the snark summary
        let result = actor.log(&snark_summary).await;

        // Verify successful database insertion
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_snark_summary_persistence_actor_handles_multiple_events() {
        let (actor, mut receiver) = setup_actor().await;

        let summaries = vec![
            SnarkCanonicitySummaryPayload {
                height: 20,
                state_hash: "hash_1".to_string(),
                timestamp: 111111,
                prover: "prover_1".to_string(),
                fee_nanomina: 1000000000,
                canonical: true,
            },
            SnarkCanonicitySummaryPayload {
                height: 21,
                state_hash: "hash_2".to_string(),
                timestamp: 222222,
                prover: "prover_2".to_string(),
                fee_nanomina: 2000000000,
                canonical: false,
            },
        ];

        for summary in &summaries {
            let event = Event {
                event_type: Event::SnarkCanonicitySummary,
                payload: sonic_rs::to_string(&summary).unwrap(),
            };
            actor.handle_event(event).await;
        }

        // Verify ActorHeight events for both summaries
        for summary in summaries {
            if let Ok(event) = timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
                let published_event: ActorHeightPayload = sonic_rs::from_str(&event.unwrap().payload).unwrap();
                assert_eq!(published_event.actor, actor.id());
                assert_eq!(published_event.height, summary.height);
            } else {
                panic!("Expected ActorHeight event was not published.");
            }
        }
    }
}
