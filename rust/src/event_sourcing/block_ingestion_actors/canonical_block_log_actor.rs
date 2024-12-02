use crate::{
    constants::TRANSITION_FRONTIER_DISTANCE,
    event_sourcing::{canonical_items_manager::CanonicalItemsManager, events::Event, payloads::CanonicalBlockLogPayload},
};
use futures::lock::Mutex;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::Arc;

pub struct CanonicalBlockLogActor {
    canonical_items_manager: Arc<Mutex<CanonicalItemsManager<CanonicalBlockLogPayload>>>,
}

impl Default for CanonicalBlockLogActor {
    fn default() -> Self {
        Self {
            canonical_items_manager: Arc::new(Mutex::new(CanonicalItemsManager::new((TRANSITION_FRONTIER_DISTANCE / 5usize) as u64))),
        }
    }
}

#[async_trait::async_trait]
impl Actor for CanonicalBlockLogActor {
    type Msg = Event;
    type State = ActorRef<Event>;
    type Arguments = ActorRef<Event>;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, parent: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        Ok(parent)
    }

    async fn handle(&self, _myself: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match msg {
            Event::BlockCanonicityUpdate(payload) => {
                {
                    let manager = self.canonical_items_manager.lock().await;
                    manager.add_block_canonicity_update(payload.clone()).await;
                }
                {
                    let manager = self.canonical_items_manager.lock().await;
                    for update_payload in manager.get_updates(payload.height).await.iter() {
                        state.cast(Event::CanonicalBlockLog(update_payload.clone()))?;
                    }
                    if let Err(e) = manager.prune().await {
                        eprintln!("{}", e);
                    }
                }
            }
            Event::BlockLog(event_payload) => {
                {
                    let manager = self.canonical_items_manager.lock().await;
                    manager
                        .add_item(CanonicalBlockLogPayload {
                            height: event_payload.height,
                            state_hash: event_payload.state_hash.to_string(),
                            previous_state_hash: event_payload.previous_state_hash.to_string(),
                            user_command_count: event_payload.user_command_count,
                            snark_work_count: event_payload.snark_work_count,
                            timestamp: event_payload.timestamp,
                            coinbase_receiver: event_payload.coinbase_receiver.to_string(),
                            coinbase_reward_nanomina: event_payload.coinbase_reward_nanomina,
                            global_slot_since_genesis: event_payload.global_slot_since_genesis,
                            last_vrf_output: event_payload.last_vrf_output.to_string(),
                            is_berkeley_block: event_payload.is_berkeley_block,
                            canonical: false, // default value
                        })
                        .await;
                    manager.add_items_count(event_payload.height, &event_payload.state_hash, 1).await;
                }
                {
                    let manager = self.canonical_items_manager.lock().await;
                    for update_payload in manager.get_updates(event_payload.height).await.iter() {
                        state.cast(Event::CanonicalBlockLog(update_payload.clone()))?;
                    }
                    if let Err(e) = manager.prune().await {
                        eprintln!("{}", e);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_sourcing::{
        payloads::{BlockCanonicityUpdatePayload, BlockLogPayload},
        test_utils::setup_test_actors,
    };
    use ractor::rpc::CallResult;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_canonical_block_log() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(CanonicalBlockLogActor::default()).await;

        // Add a BlockLog event
        let block_log_payload = BlockLogPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            previous_state_hash: "hash_0".to_string(),
            user_command_count: 10,
            snark_work_count: 5,
            timestamp: 1234567890,
            coinbase_receiver: "receiver".to_string(),
            coinbase_reward_nanomina: 1000,
            global_slot_since_genesis: 123,
            last_vrf_output: "vrf_output".to_string(),
            is_berkeley_block: true,
        };

        let response = actor.call(|_| Event::BlockLog(block_log_payload), Some(Duration::from_secs(1))).await?;

        match response {
            CallResult::Success(Event::BlockLog(_)) => {}
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        // Send a BlockCanonicityUpdate event
        let block_canonicity_update_payload = BlockCanonicityUpdatePayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            canonical: true,
            was_canonical: false,
        };

        let response = actor
            .call(|_| Event::BlockCanonicityUpdate(block_canonicity_update_payload), Some(Duration::from_secs(1)))
            .await?;

        match response {
            CallResult::Success(Event::CanonicalBlockLog(payload)) => {
                assert_eq!(payload.height, 1);
                assert_eq!(payload.state_hash, "hash_1");
                assert!(payload.canonical);
            }
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_canonical_block_log_different_event_order() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(CanonicalBlockLogActor::new()).await;

        // Add a BlockLog event
        let block_log_payload = BlockLogPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            previous_state_hash: "hash_0".to_string(),
            user_command_count: 10,
            snark_work_count: 5,
            timestamp: 1234567890,
            coinbase_receiver: "receiver".to_string(),
            coinbase_reward_nanomina: 1000,
            global_slot_since_genesis: 123,
            last_vrf_output: "vrf_output".to_string(),
            is_berkeley_block: true,
        };

        // Send a BlockCanonicityUpdate event first
        let block_canonicity_update_payload = BlockCanonicityUpdatePayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            canonical: true,
            was_canonical: false,
        };

        let response = actor
            .call(|_| Event::BlockCanonicityUpdate(block_canonicity_update_payload), Some(Duration::from_secs(1)))
            .await?;

        match response {
            CallResult::Success(Event::BlockCanonicityUpdate(_)) => {}
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        let response = actor.call(|_| Event::BlockLog(block_log_payload), Some(Duration::from_secs(1))).await?;

        match response {
            CallResult::Success(Event::CanonicalBlockLog(payload)) => {
                assert_eq!(payload.height, 1);
                assert_eq!(payload.state_hash, "hash_1");
                assert!(payload.canonical);
            }
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        Ok(())
    }
}
