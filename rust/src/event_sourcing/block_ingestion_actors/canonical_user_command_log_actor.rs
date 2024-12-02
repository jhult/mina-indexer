use crate::{
    constants::TRANSITION_FRONTIER_DISTANCE,
    event_sourcing::{canonical_items_manager::CanonicalItemsManager, error_unhandled_event, events::Event, payloads::*},
};
use futures::lock::Mutex;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::Arc;

pub struct CanonicalUserCommandLogActor {
    canonical_items_manager: Arc<Mutex<CanonicalItemsManager<CanonicalUserCommandLogPayload>>>,
}

impl Default for CanonicalUserCommandLogActor {
    fn default() -> Self {
        Self {
            canonical_items_manager: Arc::new(Mutex::new(CanonicalItemsManager::new((TRANSITION_FRONTIER_DISTANCE / 5usize) as u64))),
        }
    }
}

#[async_trait::async_trait]
impl Actor for CanonicalUserCommandLogActor {
    type Msg = Event;
    type State = Self;
    type Arguments = ();

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, parent: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        Ok(parent)
    }

    async fn handle(&self, ctx: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match msg {
            Event::BlockCanonicityUpdate(payload) => {
                {
                    let manager = self.canonical_items_manager.lock().await;
                    manager.add_block_canonicity_update(payload.clone()).await;
                }
                {
                    let manager = self.canonical_items_manager.lock().await;
                    for update_payload in manager.get_updates(payload.height).await.iter() {
                        state.cast(Event::CanonicalUserCommandLog(update_payload.clone()))?;
                    }

                    if let Err(e) = manager.prune().await {
                        eprintln!("{}", e);
                    }
                }
            }
            Event::MainnetBlock(event_payload) => {
                let manager = self.canonical_items_manager.lock().await;
                manager
                    .add_items_count(event_payload.height, &event_payload.state_hash, event_payload.user_command_count as u64)
                    .await;
            }
            Event::UserCommandLog(event_payload) => {
                {
                    let manager = self.canonical_items_manager.lock().await;
                    manager
                        .add_item(CanonicalUserCommandLogPayload {
                            height: event_payload.height,
                            txn_hash: event_payload.txn_hash.to_string(),
                            state_hash: event_payload.state_hash.to_string(),
                            timestamp: event_payload.timestamp,
                            txn_type: event_payload.txn_type.clone(),
                            status: event_payload.status.clone(),
                            sender: event_payload.sender.to_string(),
                            receiver: event_payload.receiver.to_string(),
                            nonce: event_payload.nonce,
                            fee_nanomina: event_payload.fee_nanomina,
                            fee_payer: event_payload.fee_payer.to_string(),
                            amount_nanomina: event_payload.amount_nanomina,
                            global_slot: event_payload.global_slot,
                            canonical: false,     // use a default value
                            was_canonical: false, // use a default value
                        })
                        .await;
                }
                {
                    let manager = self.canonical_items_manager.lock().await;
                    for update_payload in manager.get_updates(event_payload.height).await.iter() {
                        state.cast(Event::CanonicalUserCommandLog(update_payload.clone()))?;
                    }

                    if let Err(e) = manager.prune().await {
                        eprintln!("{}", e);
                    }
                }
            }
            _ => error_unhandled_event(&msg),
        }
        Ok(())
    }
}
#[cfg(test)]
mod canonical_user_command_log_actor_tests {
    use super::*;
    use crate::event_sourcing::{
        events::Event,
        mainnet_block_models::{CommandStatus, CommandType},
        payloads::{BlockCanonicityUpdatePayload, MainnetBlockPayload, UserCommandLogPayload},
        test_utils::setup_test_actors,
    };
    use ractor::rpc::CallResult;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_publishes_after_all_conditions_met() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(CanonicalUserCommandLogActor::default()).await;

        // Add a mainnet block with user command count
        let mainnet_block = MainnetBlockPayload {
            height: 10,
            user_command_count: 2,
            state_hash: "state_hash_10".to_string(),
            ..Default::default()
        };
        actor.call(|_| Event::MainnetBlock(mainnet_block.clone()), Some(Duration::from_secs(1))).await?;

        // Add two user command logs
        let user_command_1 = UserCommandLogPayload {
            height: 10,
            global_slot: 0,
            state_hash: "state_hash_10".to_string(),
            txn_hash: "txn_hash_1".to_string(),
            timestamp: 123456,
            txn_type: CommandType::Payment,
            status: CommandStatus::Applied,
            sender: "sender_1".to_string(),
            receiver: "receiver_1".to_string(),
            nonce: 1,
            fee_nanomina: 1000,
            fee_payer: "payer_1".to_string(),
            amount_nanomina: 5000,
        };
        let user_command_2 = UserCommandLogPayload {
            height: 10,
            global_slot: 0,
            state_hash: "state_hash_10".to_string(),
            txn_hash: "txn_hash_2".to_string(),
            timestamp: 123456,
            txn_type: CommandType::StakeDelegation,
            status: CommandStatus::Applied,
            sender: "sender_2".to_string(),
            receiver: "receiver_2".to_string(),
            nonce: 2,
            fee_nanomina: 2000,
            fee_payer: "payer_2".to_string(),
            amount_nanomina: 7000,
        };
        actor
            .call(|_| Event::UserCommandLog(user_command_1.clone()), Some(Duration::from_secs(1)))
            .await?;
        actor
            .call(|_| Event::UserCommandLog(user_command_2.clone()), Some(Duration::from_secs(1)))
            .await?;

        // Add a block canonicity update for the same height
        let update = BlockCanonicityUpdatePayload {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            canonical: true,
            was_canonical: false,
        };
        let response = actor
            .call(|_| Event::BlockCanonicityUpdate(update.clone()), Some(Duration::from_secs(1)))
            .await?;

        // Verify the response
        match response {
            CallResult::Success(Event::CanonicalUserCommandLog(payload)) => {
                assert_eq!(payload.height, 10);
                assert_eq!(payload.state_hash, "state_hash_10");
                assert!(payload.canonical);
            }
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_does_not_publish_without_all_conditions_met() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(CanonicalUserCommandLogActor::default()).await;

        // Add a mainnet block with user command count
        let mainnet_block = MainnetBlockPayload {
            height: 10,
            user_command_count: 2,
            state_hash: "state_hash_other".to_string(), // no matching state hash
            ..Default::default()
        };
        actor.call(|_| Event::MainnetBlock(mainnet_block.clone()), Some(Duration::from_secs(1))).await?;

        // Add one user command log (not enough)
        let user_command = UserCommandLogPayload {
            height: 10,
            global_slot: 0,
            state_hash: "state_hash_10".to_string(),
            txn_hash: "txn_hash_1".to_string(),
            timestamp: 123456,
            txn_type: CommandType::Payment,
            status: CommandStatus::Applied,
            sender: "sender_1".to_string(),
            receiver: "receiver_1".to_string(),
            nonce: 1,
            fee_nanomina: 1000,
            fee_payer: "payer_1".to_string(),
            amount_nanomina: 5000,
        };
        actor
            .call(|_| Event::UserCommandLog(user_command.clone()), Some(Duration::from_secs(1)))
            .await?;

        // Add a block canonicity update for the same height
        let update = BlockCanonicityUpdatePayload {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            canonical: true,
            was_canonical: false,
        };
        let response = actor
            .call(|_| Event::BlockCanonicityUpdate(update.clone()), Some(Duration::from_secs(1)))
            .await?;

        // Verify the response
        match response {
            CallResult::Success(_) => panic!("Unexpected message type received"),
            CallResult::Timeout => {
                // Expected outcome: no event should be published
            }
            CallResult::SenderError => panic!("Call failed"),
        }

        Ok(())
    }
}
