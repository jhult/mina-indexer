use crate::{
    blockchain_tree::{BlockchainTree, Hash, Height, Node},
    event_sourcing::{events::Event, payloads::BlockConfirmationPayload},
};
use futures::lock::Mutex;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::Arc;

pub struct BlockConfirmationsActor {
    blockchain_tree: Arc<Mutex<BlockchainTree>>,
}

impl Default for BlockConfirmationsActor {
    fn default() -> Self {
        Self {
            blockchain_tree: Arc::new(Mutex::new(BlockchainTree::new(11))),
        }
    }
}

#[async_trait::async_trait]
impl Actor for BlockConfirmationsActor {
    type Msg = Event;
    type State = ActorRef<Event>;
    type Arguments = ActorRef<Event>;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, parent: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        Ok(parent)
    }

    async fn handle(&self, _myself: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        if let Event::NewBlock(block_payload) = msg {
            let mut blockchain_tree = self.blockchain_tree.lock().await;
            let next_node = Node {
                height: Height(block_payload.height),
                state_hash: Hash(block_payload.state_hash),
                previous_state_hash: Hash(block_payload.previous_state_hash),
                last_vrf_output: block_payload.last_vrf_output,
                metadata_str: Some("0".to_string()),
            };

            if blockchain_tree.is_empty() {
                blockchain_tree.set_root(next_node)?;
                return Ok(());
            } else if blockchain_tree.has_parent(&next_node) {
                blockchain_tree.add_node(next_node.clone())?;
                let mut iter_node = next_node;

                while let Some(parent) = blockchain_tree.get_parent_mut(&iter_node) {
                    if let Some(confirmations_str) = parent.metadata_str.clone() {
                        // Parse, increment, and update the confirmations count
                        let new_confirmations = confirmations_str.parse::<u8>().unwrap_or(0) + 1;
                        parent.metadata_str = Some(new_confirmations.to_string());

                        // Publish the confirmation event
                        if new_confirmations == 10 {
                            let confirmation = BlockConfirmationPayload {
                                height: parent.height.0,
                                state_hash: parent.state_hash.0.clone(),
                                confirmations: new_confirmations,
                            };
                            state.cast(Event::BlockConfirmation(confirmation))?;
                        }
                    }
                    iter_node = parent.clone();
                }
                blockchain_tree.prune_tree()?;
            }
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_block_confirmations_actor() -> anyhow::Result<()> {
    use crate::event_sourcing::{payloads::NewBlockPayload, test_utils::setup_test_actors};
    use ractor::rpc::CallResult;
    use tokio::time::Duration;

    // Setup test actors
    let actor = setup_test_actors(BlockConfirmationsActor::default()).await;

    // Add blocks 0-10 to build up the chain
    for i in 0..11 {
        let block_payload = NewBlockPayload {
            height: i,
            state_hash: format!("hash_{}", i),
            previous_state_hash: if i == 0 { "".to_string() } else { format!("hash_{}", i - 1) },
            last_vrf_output: format!("vrf_output_{}", i),
        };

        let response = actor.call(|_| Event::NewBlock(block_payload), Some(Duration::from_secs(1))).await?;

        match response {
            CallResult::Success(_) => {}
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }
    }

    // Add block 11 which should trigger confirmation for block 1
    let block_payload = NewBlockPayload {
        height: 11,
        state_hash: "hash_11".to_string(),
        previous_state_hash: "hash_10".to_string(),
        last_vrf_output: "vrf_output_11".to_string(),
    };

    let response = actor.call(|_| Event::NewBlock(block_payload), Some(Duration::from_secs(1))).await?;

    // Verify the confirmation event
    match response {
        CallResult::Success(Event::BlockConfirmation(payload)) => {
            assert_eq!(payload.height, 1);
            assert_eq!(payload.state_hash, "hash_1");
            assert_eq!(payload.confirmations, 10);
        }
        CallResult::Success(_) => panic!("Received unexpected message type"),
        CallResult::Timeout => panic!("Call timed out"),
        CallResult::SenderError => panic!("Call failed"),
    }

    Ok(())
}

#[tokio::test]
async fn test_add_root_node() -> anyhow::Result<()> {
    use crate::event_sourcing::{payloads::NewBlockPayload, test_utils::setup_test_actors};
    use ractor::rpc::CallResult;
    use tokio::time::Duration;

    // Setup test actors
    let actor = setup_test_actors(BlockConfirmationsActor::new()).await;

    let payload = NewBlockPayload {
        height: 0,
        state_hash: "root_hash".to_string(),
        previous_state_hash: "".to_string(),
        last_vrf_output: "vrf_output".to_string(),
    };

    let response = actor.call(|_| Event::NewBlock(payload), Some(Duration::from_secs(1))).await?;

    // No confirmations should be published for the root node
    match response {
        CallResult::Success(Event::NewBlock(_)) => {}
        CallResult::Success(_) => panic!("Received unexpected message type"),
        CallResult::Timeout => panic!("Call timed out"),
        CallResult::SenderError => panic!("Call failed"),
    }

    Ok(())
}
