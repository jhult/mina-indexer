use super::super::events::Event;
use crate::{
    blockchain_tree::Height,
    constants::POSTGRES_CONNECTION_STRING,
    stream::payloads::{BlockConfirmationPayload, MainnetBlockPayload, NewAccountPayload},
};
use futures::lock::Mutex;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::{collections::HashMap, sync::Arc};
use tokio_postgres::{Client, NoTls};

pub struct NewAccountActor {
    pub mainnet_blocks: Arc<Mutex<HashMap<Height, Vec<MainnetBlockPayload>>>>,
    pub client: Client,
}

#[async_trait::async_trait]
impl Actor for NewAccountActor {
    type Msg = Event;
    type State = ActorRef<Event>;
    type Arguments = ActorRef<Event>;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, parent: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        if let Ok((client, connection)) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls).await {
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("connection error: {}", e);
                }
            });

            // Handle table setup
            if let Err(e) = client.execute("DROP TABLE IF EXISTS discovered_accounts;", &[]).await {
                println!("Unable to drop discovered_accounts table {:?}", e);
            }

            if let Err(e) = client
                .execute(
                    "CREATE TABLE IF NOT EXISTS discovered_accounts (
                            account TEXT PRIMARY KEY NOT NULL,
                            height BIGINT NOT NULL
                        );",
                    &[],
                )
                .await
            {
                println!("Unable to create discovered_accounts table {:?}", e);
            }

            Ok(Self {
                mainnet_blocks: Arc::new(Mutex::new(HashMap::new())),
                client,
            })
        } else {
            Err(ActorProcessingErr::from("Unable to establish connection to database".into()))
        }
    }

    async fn handle(&self, _myself: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match msg {
            Event::PreExistingAccount(account) => {
                // Insert the account into the `discovered_accounts` table
                let insert_query = "INSERT INTO discovered_accounts (account, height) VALUES ($1, $2) ON CONFLICT DO NOTHING";
                if let Err(e) = self.client.execute(insert_query, &[&account, &0_i64]).await {
                    eprintln!("Failed to insert account {} into database: {:?}", account, e);
                } else {
                    self.database_inserts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
            }
            Event::MainnetBlock(block) => {
                let mut mainnet_blocks = self.mainnet_blocks.lock().await;
                mainnet_blocks.entry(Height(block.height)).or_insert_with(Vec::new).push(block);
            }
            Event::BlockConfirmation(block_confirmation) => {
                if block_confirmation.confirmations == 10 {
                    let mut mainnet_blocks = self.mainnet_blocks.lock().await;

                    // Look up the blocks at the confirmed height
                    if let Some(blocks) = mainnet_blocks.remove(&Height(block_confirmation.height)) {
                        for block in blocks {
                            for account in block.valid_accounts().iter().filter(|a| !a.is_empty()) {
                                if block.state_hash == block_confirmation.state_hash {
                                    // Check if the account is already in the database
                                    let check_query = "SELECT EXISTS (SELECT 1 FROM discovered_accounts WHERE account = $1)";

                                    let account_check = self
                                        .client
                                        .query_one(check_query, &[&account])
                                        .await
                                        .map(|row| row.get::<_, bool>(0))
                                        .unwrap_or(false);

                                    if !account_check {
                                        // Publish a NewAccount event
                                        let new_account_event = Event::NewAccount(NewAccountPayload {
                                            height: block.height,
                                            state_hash: block.state_hash.clone(),
                                            timestamp: block.timestamp,
                                            account: account.clone(),
                                        });
                                        state.cast(new_account_event);

                                        // Insert the account into the database
                                        let insert_query = "INSERT INTO discovered_accounts (account, height) VALUES ($1, $2)";
                                        if let Err(e) = self.client.execute(insert_query, &[&account, &(block_confirmation.height as i64)]).await {
                                            eprintln!("Failed to insert new account into database: {:?}", e);
                                        } else {
                                            self.database_inserts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod new_account_actor_tests {
    use super::*;
    use crate::stream::{
        events::Event,
        mainnet_block_models::{CommandStatus, CommandSummary},
        payloads::{BlockConfirmationPayload, MainnetBlockPayload, NewAccountPayload},
    };
    use std::sync::Arc;
    use tokio::time::timeout;

    async fn setup_actor() -> (Arc<NewAccountActor>, tokio::sync::broadcast::Receiver<Event>) {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = Arc::new(NewAccountActor::new(Arc::clone(&shared_publisher), &None).await);
        let receiver = shared_publisher.subscribe();
        (actor, receiver)
    }

    #[tokio::test]
    async fn test_preexisting_account_inserted() {
        let (actor, _) = setup_actor().await;

        let account = "B62qtestaccount1".to_string();
        let event = Event {
            event_type: Event::PreExistingAccount,
            payload: account.to_string(),
        };

        actor.handle_event(event).await;

        // Verify the account is inserted in the database
        let check_query = "SELECT EXISTS (SELECT 1 FROM discovered_accounts WHERE account = $1)";
        let account_exists: bool = actor.client.query_one(check_query, &[&account]).await.unwrap().get(0);

        assert!(account_exists, "Pre-existing account should be inserted into the database");
    }

    #[tokio::test]
    async fn test_mainnet_block_handling() {
        let (actor, _) = setup_actor().await;

        let block = MainnetBlockPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            user_commands: vec![CommandSummary {
                sender: "B62qaccount1".to_string(),
                receiver: "B62qaccount2".to_string(),
                ..Default::default()
            }],
            timestamp: 1234567890,
            ..Default::default()
        };

        let event = Event {
            event_type: Event::MainnetBlock,
            payload: sonic_rs::to_string(&block).unwrap(),
        };

        actor.handle_event(event).await;

        // Verify the block is stored in the mainnet_blocks map
        let mainnet_blocks = actor.mainnet_blocks.lock().await;
        let stored_blocks = mainnet_blocks.get(&Height(block.height));
        assert!(stored_blocks.is_some(), "Mainnet block should be stored in memory");
        assert_eq!(stored_blocks.unwrap().len(), 1, "Mainnet block should contain one entry");
    }

    #[tokio::test]
    async fn test_block_confirmation_with_new_accounts() {
        let (actor, mut receiver) = setup_actor().await;

        let block = MainnetBlockPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            user_commands: vec![CommandSummary {
                sender: "B62qnewaccount".to_string(),
                receiver: "B62qnewaccount".to_string(),
                fee_payer: "B62qnewaccount".to_string(),
                status: CommandStatus::Applied,
                ..Default::default()
            }],
            timestamp: 1234567890,
            ..Default::default()
        };

        // Add the mainnet block
        let block_event = Event {
            event_type: Event::MainnetBlock,
            payload: sonic_rs::to_string(&block).unwrap(),
        };
        actor.handle_event(block_event).await;

        // Confirm the block
        let confirmation_payload = BlockConfirmationPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            confirmations: 10,
        };

        let confirmation_event = Event {
            event_type: Event::BlockConfirmation,
            payload: sonic_rs::to_string(&confirmation_payload).unwrap(),
        };

        actor.handle_event(confirmation_event).await;

        // Verify a NewAccount event is published
        if let Ok(event) = timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
            let received_event = event.unwrap();
            assert_eq!(received_event.event_type, Event::NewAccount);

            let new_account_payload: NewAccountPayload = sonic_rs::from_str(&received_event.payload).unwrap();
            assert_eq!(new_account_payload.height, block.height);
            assert_eq!(new_account_payload.account, "B62qnewaccount".to_string());
        } else {
            panic!("Expected NewAccount event not received");
        }
    }

    #[tokio::test]
    async fn test_block_confirmation_with_new_accounts_failed_command() {
        let (actor, mut receiver) = setup_actor().await;

        let block = MainnetBlockPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            user_commands: vec![CommandSummary {
                sender: "B62qnewaccount".to_string(),
                receiver: "B62qnewaccount".to_string(),
                fee_payer: "B62qnewaccount".to_string(),
                status: CommandStatus::Failed,
                ..Default::default()
            }],
            timestamp: 1234567890,
            ..Default::default()
        };

        // Add the mainnet block
        let block_event = Event {
            event_type: Event::MainnetBlock,
            payload: sonic_rs::to_string(&block).unwrap(),
        };
        actor.handle_event(block_event).await;

        // Confirm the block
        let confirmation_payload = BlockConfirmationPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            confirmations: 10,
        };

        let confirmation_event = Event {
            event_type: Event::BlockConfirmation,
            payload: sonic_rs::to_string(&confirmation_payload).unwrap(),
        };

        actor.handle_event(confirmation_event).await;

        let published_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(published_event.is_err(), "Expect failed user command to not publish event");
    }

    #[tokio::test]
    async fn test_block_confirmation_with_existing_account() {
        let (actor, mut receiver) = setup_actor().await;

        let account = "B62qexistingaccount".to_string();

        // Add the preexisting account to the database
        let preexisting_event = Event {
            event_type: Event::PreExistingAccount,
            payload: account.to_string(),
        };
        actor.handle_event(preexisting_event).await;

        let block = MainnetBlockPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            user_commands: vec![CommandSummary {
                sender: account.clone(),
                ..Default::default()
            }],
            timestamp: 1234567890,
            ..Default::default()
        };

        // Add the mainnet block
        let block_event = Event {
            event_type: Event::MainnetBlock,
            payload: sonic_rs::to_string(&block).unwrap(),
        };
        actor.handle_event(block_event).await;

        // Confirm the block
        let confirmation_payload = BlockConfirmationPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            confirmations: 10,
        };

        let confirmation_event = Event {
            event_type: Event::BlockConfirmation,
            payload: sonic_rs::to_string(&confirmation_payload).unwrap(),
        };

        actor.handle_event(confirmation_event).await;

        // Filter the events to ensure no NewAccount event is published for the sender
        let mut received_events = vec![];
        while let Ok(event) = timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
            received_events.push(event.unwrap());
        }

        // Check for NewAccount events and ensure none is published for the sender
        let new_account_events: Vec<_> = received_events
            .iter()
            .filter(|event| {
                let event_payload: NewAccountPayload = sonic_rs::from_str(&event.payload).unwrap();
                event_payload.account == account
            })
            .collect();

        assert!(new_account_events.is_empty(), "No NewAccount event should be published for existing accounts");
    }

    #[tokio::test]
    async fn test_discovered_accounts_pruned_above_root_height() {
        use std::sync::Arc;

        // Step 1: Initialize the actor without a root node and add accounts at various heights
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor_without_root = NewAccountActor::new(Arc::clone(&shared_publisher), &None).await;

        // Insert accounts at different heights
        let accounts = vec![("B62qAccountAtHeight1", 1), ("B62qAccountAtHeight10", 10), ("B62qAccountAtHeight15", 15)];

        for (account, height) in &accounts {
            actor_without_root
                .client
                .execute(
                    "INSERT INTO discovered_accounts (account, height) VALUES ($1, $2)",
                    &[account, &(*height as i64)],
                )
                .await
                .expect("Failed to insert account");
        }

        // Step 2: Initialize the actor with a root node at height 10
        let root_node_height = 10;
        let root_node = Some((root_node_height, "root_hash".to_string()));
        let actor_with_root = NewAccountActor::new(Arc::clone(&shared_publisher), &root_node).await;

        // Query and manually check each account
        let check_query = "SELECT EXISTS (SELECT 1 FROM discovered_accounts WHERE account = $1)";

        // Account at height 1 should remain
        let account_at_1 = "B62qAccountAtHeight1";
        let exists_at_1: bool = actor_with_root
            .client
            .query_one(check_query, &[&account_at_1])
            .await
            .expect("Failed to query database for account at height 1")
            .get(0);
        assert!(exists_at_1, "Account at height 1 should remain");

        // Account at height 10 should be deleted
        let account_at_10 = "B62qAccountAtHeight10";
        let exists_at_10: bool = actor_with_root
            .client
            .query_one(check_query, &[&account_at_10])
            .await
            .expect("Failed to query database for account at height 10")
            .get(0);
        assert!(!exists_at_10, "Account at height 10 should be deleted since it matches the root height");

        // Account at height 15 should be deleted
        let account_at_15 = "B62qAccountAtHeight15";
        let exists_at_15: bool = actor_with_root
            .client
            .query_one(check_query, &[&account_at_15])
            .await
            .expect("Failed to query database for account at height 15")
            .get(0);
        assert!(!exists_at_15, "Account at height 15 should be deleted since it is above the root height");
    }
}
