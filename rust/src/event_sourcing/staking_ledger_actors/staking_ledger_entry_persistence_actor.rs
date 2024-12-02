use super::super::events::Event;
use crate::{constants::POSTGRES_CONNECTION_STRING, event_sourcing::payloads::StakingLedgerEntryPayload};
use anyhow::Result;
use async_trait::async_trait;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use tokio_postgres::{Client, NoTls};

pub struct StakingLedgerEntryPersistenceActor {
    pub client: Client,
}

impl Default for StakingLedgerEntryPersistenceActor {
    fn default() -> Self {
        let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Unable to establish connection to database");

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        if let Err(e) = client.execute("DROP TABLE IF EXISTS staking_ledger CASCADE;", &[]).await {
            eprintln!("Unable to drop staking_ledger table {e}");
        }

        let table_create = r#"
            CREATE TABLE IF NOT EXISTS staking_ledger (
                entry_id BIGSERIAL PRIMARY KEY,
                epoch BIGINT,
                delegate TEXT,
                stake BIGINT,
                total_staked BIGINT,
                delegators_count BIGINT
            );
        "#;
        if let Err(e) = client.execute(table_create, &[]).await {
            eprintln!("Unable to create staking_ledger table {e}");
        }

        Self { client }
    }
}

#[async_trait]
impl Actor for StakingLedgerEntryPersistenceActor {
    type Msg = Event;
    type State = ();
    type Arguments = ();

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, _args: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        Ok(())
    }

    async fn handle(&self, _myself: ActorRef<Self::Msg>, msg: Self::Msg, _state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        if let Event::StakingLedgerEntry(payload) = msg {
            self.insert(&payload).await?;
        }
        Ok(())
    }
}

impl StakingLedgerEntryPersistenceActor {
    async fn insert(&self, payload: &StakingLedgerEntryPayload) -> Result<(), &'static str> {
        self.client
            .execute(
                r#"
                INSERT INTO staking_ledger (epoch, delegate, stake, total_staked, delegators_count)
                VALUES ($1, $2, $3, $4, $5);
                "#,
                &[
                    &(payload.epoch as i64),
                    &payload.delegate,
                    &(payload.stake as i64),
                    &(payload.total_staked as i64),
                    &(payload.delegators_count as i64),
                ],
            )
            .await
            .map_err(|e| {
                eprintln!("Database insert error: {}", e);
                "Unable to insert into staking_ledger table"
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod staking_ledger_entry_persistence_actor_tests {
    use super::*;
    use crate::event_sourcing::events::Event;

    #[tokio::test]
    async fn test_persistence_of_staking_ledger_entry() {
        let actor = StakingLedgerEntryPersistenceActor::default();

        let payload = StakingLedgerEntryPayload {
            epoch: 10,
            delegate: "delegate_1".to_string(),
            stake: 1000000,
            total_staked: 5000000,
            delegators_count: 20,
        };

        actor
            .handle(ActorRef::default(), Event::StakingLedgerEntry(payload.clone()), &mut ())
            .await
            .unwrap();
    }
}
