use crate::{
    blockchain_tree::{BlockchainTree, Hash, Height, Node},
    event_sourcing::{events::Event, payloads::NewBlockPayload},
};
use futures::lock::Mutex;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::Arc;

pub struct BlockchainTreeBuilderActor {
    blockchain_tree: Arc<Mutex<BlockchainTree>>,
}

impl Default for BlockchainTreeBuilderActor {
    fn default() -> Self {
        Self {
            blockchain_tree: Arc::new(Mutex::new(BlockchainTree::new(11))),
        }
    }
}

#[async_trait::async_trait]
impl Actor for BlockchainTreeBuilderActor {
    type Msg = Event;
    type State = ActorRef<Event>;
    type Arguments = ActorRef<Event>;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, parent: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        Ok(parent)
    }

    async fn handle(&self, myself: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match msg {
            Event::BlockAncestor(block_payload) => {
                let mut blockchain_tree = self.blockchain_tree.lock().await;
                let next_node = Node {
                    height: Height(block_payload.height),
                    state_hash: Hash(block_payload.state_hash.clone()),
                    previous_state_hash: Hash(block_payload.previous_state_hash.clone()),
                    last_vrf_output: block_payload.last_vrf_output.clone(),
                    ..Default::default()
                };

                if blockchain_tree.is_empty() {
                    // we assume the first block we receive is carefully selected
                    // to be the best canonical tip
                    blockchain_tree.set_root(next_node.clone())?;
                } else if blockchain_tree.has_parent(&next_node) {
                    blockchain_tree.add_node(next_node.clone())?;
                } else {
                    // try again later
                    state.cast(Event::BlockAncestor(block_payload))?;
                    return Ok(());
                }

                let added_payload = NewBlockPayload {
                    height: block_payload.height,
                    state_hash: block_payload.state_hash,
                    previous_state_hash: block_payload.previous_state_hash,
                    last_vrf_output: block_payload.last_vrf_output,
                };
                state.cast(Event::NewBlock(added_payload))?;
                blockchain_tree.prune_tree()?;
            }
            Event::GenesisBlock(genesis_payload) => {
                let mut blockchain_tree = self.blockchain_tree.lock().await;
                let root_node = Node {
                    height: Height(genesis_payload.height),
                    state_hash: Hash(genesis_payload.state_hash.clone()),
                    previous_state_hash: Hash(genesis_payload.previous_state_hash.clone()),
                    last_vrf_output: genesis_payload.last_vrf_output.clone(),
                    ..Default::default()
                };
                blockchain_tree.set_root(root_node)?;

                let added_payload = NewBlockPayload {
                    height: genesis_payload.height,
                    state_hash: genesis_payload.state_hash,
                    previous_state_hash: genesis_payload.previous_state_hash,
                    last_vrf_output: genesis_payload.last_vrf_output,
                };
                state.cast(Event::NewBlock(added_payload))?;
            }
            Event::NewBlock(block_payload) => {
                let mut blockchain_tree = self.blockchain_tree.lock().await;
                let next_node = Node {
                    height: Height(block_payload.height),
                    state_hash: Hash(block_payload.state_hash),
                    previous_state_hash: Hash(block_payload.previous_state_hash),
                    last_vrf_output: block_payload.last_vrf_output,
                    ..Default::default()
                };

                if blockchain_tree.is_empty() {
                    blockchain_tree.set_root(next_node)?;
                } else if blockchain_tree.has_parent(&next_node) {
                    blockchain_tree.add_node(next_node)?;
                    blockchain_tree.prune_tree()?;
                } else {
                    println!(
                        "Attempted to add block at height {} and state_hash {} but found no parent",
                        next_node.height.0, next_node.state_hash.0
                    );
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
        payloads::{BlockAncestorPayload, GenesisBlockPayload, NewBlockPayload},
        test_utils::setup_test_actors,
    };
    use ractor::rpc::CallResult;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_blockchain_tree_builder_actor() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(BlockchainTreeBuilderActor::new()).await;

        // Create test payloads
        let block_payloads = vec![
            NewBlockPayload {
                height: 0,
                state_hash: "3N8aBlock1".to_string(),
                previous_state_hash: "".to_string(),
                last_vrf_output: "vrf_output_0".to_string(),
            },
            NewBlockPayload {
                height: 1,
                state_hash: "3N8aBlock2".to_string(),
                previous_state_hash: "3N8aBlock1".to_string(),
                last_vrf_output: "vrf_output_1".to_string(),
            },
            NewBlockPayload {
                height: 2,
                state_hash: "3N8aBlock3".to_string(),
                previous_state_hash: "3N8aBlock2".to_string(),
                last_vrf_output: "vrf_output_2".to_string(),
            },
        ];

        // Send test messages and verify responses
        for block_payload in block_payloads {
            let response = actor.call(|_| Event::NewBlock(block_payload.clone()), Some(Duration::from_secs(1))).await?;

            match response {
                CallResult::Success(Event::NewBlock(_)) => {}
                CallResult::Success(_) => panic!("Received unexpected message type"),
                CallResult::Timeout => panic!("Call timed out"),
                CallResult::SenderError => panic!("Call failed"),
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_blockchain_tree_builder_invalid_parent() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(BlockchainTreeBuilderActor::new()).await;

        // Create test payload with invalid parent
        let invalid_block = NewBlockPayload {
            height: 1,
            state_hash: "3N8aInvalidBlock".to_string(),
            previous_state_hash: "3N8aNonexistentParent".to_string(),
            last_vrf_output: "vrf_output".to_string(),
        };

        // Send test message and verify response
        let response = actor.call(|_| Event::NewBlock(invalid_block), Some(Duration::from_secs(1))).await?;

        match response {
            CallResult::Success(Event::NewBlock(_)) => {}
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_blockchain_tree_builder_genesis_block() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(BlockchainTreeBuilderActor::new()).await;

        let genesis_block = GenesisBlockPayload::default();

        let response = actor.call(|_| Event::GenesisBlock(genesis_block), Some(Duration::from_secs(1))).await?;

        match response {
            CallResult::Success(Event::NewBlock(_)) => {}
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_blockchain_tree_builder_block_ancestor() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(BlockchainTreeBuilderActor::new()).await;

        let block_ancestor = BlockAncestorPayload {
            height: 1,
            state_hash: "3N8aAncestorBlock".to_string(),
            previous_state_hash: "3N8aGenesisBlock".to_string(),
            last_vrf_output: "vrf_output".to_string(),
        };

        let response = actor.call(|_| Event::BlockAncestor(block_ancestor), Some(Duration::from_secs(1))).await?;

        match response {
            CallResult::Success(Event::NewBlock(_)) => {}
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_blockchain_tree_actor_publishes_root() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(BlockchainTreeBuilderActor::new()).await;

        let block_ancestor = BlockAncestorPayload {
            height: 1,
            state_hash: "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ".to_string(),
            previous_state_hash: "3NLoKn22eMnyQ7rxh5pxB6vBA3XhSAhhrf7akdqS6HbAKD14Dh1d".to_string(),
            last_vrf_output: "NfThG1r1GxQuhaGLSJWGxcpv24SudtXG4etB0TnGqwg=".to_string(),
        };

        let response = actor
            .call(|_| Event::BlockAncestor(block_ancestor.clone()), Some(Duration::from_secs(1)))
            .await?;

        match response {
            CallResult::Success(Event::NewBlock(payload)) => {
                assert_eq!(payload.height, block_ancestor.height);
                assert_eq!(payload.state_hash, block_ancestor.state_hash);
                assert_eq!(payload.previous_state_hash, block_ancestor.previous_state_hash);
                assert_eq!(payload.last_vrf_output, block_ancestor.last_vrf_output);
            }
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_blockchain_tree_actor_reconnects_when_ancestor_arrives() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(BlockchainTreeBuilderActor::new()).await;

        let block_ancestor_1 = BlockAncestorPayload {
            height: 2,
            state_hash: "3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH".to_string(),
            previous_state_hash: "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ".to_string(),
            last_vrf_output: "vrf_output_1".to_string(),
        };

        let block_ancestor_2 = BlockAncestorPayload {
            height: 1,
            state_hash: "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ".to_string(),
            previous_state_hash: "3NLoKn22eMnyQ7rxh5pxB6vBA3XhSAhhrf7akdqS6HbAKD14Dh1d".to_string(),
            last_vrf_output: "NfThG1r1GxQuhaGLSJWGxcpv24SudtXG4etB0TnGqwg=".to_string(),
        };

        // Send block_ancestor_1 first (should be retried as it has no parent)
        let response = actor
            .call(|_| Event::BlockAncestor(block_ancestor_1.clone()), Some(Duration::from_secs(1)))
            .await?;

        // Verify it was retried
        match response {
            CallResult::Success(Event::BlockAncestor(_)) => {}
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        // Send block_ancestor_2 (parent block)
        let response = actor.call(|_| Event::BlockAncestor(block_ancestor_2), Some(Duration::from_secs(1))).await?;

        // Verify block_ancestor_2 was added
        match response {
            CallResult::Success(Event::NewBlock(payload)) => {
                assert_eq!(payload.height, 1);
                assert_eq!(payload.state_hash, "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ");
            }
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        // Send block_ancestor_1 again (should now succeed as parent exists)
        let response = actor.call(|_| Event::BlockAncestor(block_ancestor_1), Some(Duration::from_secs(1))).await?;

        // Verify block_ancestor_1 was added
        match response {
            CallResult::Success(Event::NewBlock(payload)) => {
                assert_eq!(payload.height, 2);
                assert_eq!(payload.state_hash, "3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH");
            }
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_blockchain_tree_builder_pruning() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(BlockchainTreeBuilderActor::new()).await;

        // Create a chain of 15 blocks (exceeding the prune limit of 11)
        for i in 0..15 {
            let block = NewBlockPayload {
                height: i,
                state_hash: format!("3N8aBlock{}", i),
                previous_state_hash: if i == 0 { "".to_string() } else { format!("3N8aBlock{}", i - 1) },
                last_vrf_output: format!("vrf_{}", i),
            };

            let response = actor.call(|_| Event::NewBlock(block), Some(Duration::from_secs(1))).await?;

            match response {
                CallResult::Success(Event::NewBlock(_)) => {}
                CallResult::Success(_) => panic!("Received unexpected message type"),
                CallResult::Timeout => panic!("Call timed out"),
                CallResult::SenderError => panic!("Call failed"),
            }
        }

        Ok(())
    }
}
