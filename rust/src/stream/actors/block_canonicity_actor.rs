use crate::{
    blockchain_tree::{BlockchainTree, Hash, Height, Node},
    constants::TRANSITION_FRONTIER_DISTANCE,
    stream::{events::Event, payloads::BlockCanonicityUpdatePayload},
};
use futures::lock::Mutex;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::Arc;

pub struct BlockCanonicityActor {
    blockchain_tree: Arc<Mutex<BlockchainTree>>,
}

impl BlockCanonicityActor {
    pub fn new() -> Self {
        Self {
            blockchain_tree: Arc::new(Mutex::new(BlockchainTree::new(TRANSITION_FRONTIER_DISTANCE))),
        }
    }

    async fn process_equal_height(
        &self,
        state: &ActorRef<Event>,
        blockchain_tree: &mut BlockchainTree,
        current_best_block: &Node,
        next_node: Node,
    ) -> Result<(), ActorProcessingErr> {
        if BlockchainTree::greater(&next_node, current_best_block) {
            self.update_ancestries(state, blockchain_tree, current_best_block, &next_node).await?;
        } else {
            self.publish_canonical_update(state, next_node, false, false)?;
        }
        Ok(())
    }

    async fn process_greater_height(
        &self,
        state: &ActorRef<Event>,
        blockchain_tree: &mut BlockchainTree,
        current_best_block: &Node,
        next_node: Node,
    ) -> Result<(), ActorProcessingErr> {
        let parent = blockchain_tree.get_parent(&next_node).unwrap();
        if parent.state_hash != current_best_block.state_hash {
            self.update_ancestries(state, blockchain_tree, current_best_block, parent).await?;
        }
        self.publish_canonical_update(state, next_node, true, false)?;
        Ok(())
    }

    async fn update_ancestries(
        &self,
        state: &ActorRef<Event>,
        blockchain_tree: &BlockchainTree,
        current_best_block: &Node,
        next_node: &Node,
    ) -> Result<(), ActorProcessingErr> {
        let (prior_ancestry, mut new_ancestry, _) = blockchain_tree.get_shared_ancestry(current_best_block, next_node).unwrap();

        for prior in prior_ancestry.iter() {
            self.publish_canonical_update(state, prior.clone(), false, true)?;
        }

        new_ancestry.reverse();
        for new_a in new_ancestry.iter() {
            self.publish_canonical_update(state, new_a.clone(), true, false)?;
        }
        Ok(())
    }

    fn publish_canonical_update(&self, state: &ActorRef<Event>, node: Node, canonical: bool, was_canonical: bool) -> Result<(), ActorProcessingErr> {
        let update = BlockCanonicityUpdatePayload {
            height: node.height.0,
            state_hash: node.state_hash.0,
            canonical,
            was_canonical,
        };
        state.cast(Event::BlockCanonicityUpdate(update))?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl Actor for BlockCanonicityActor {
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
                ..Default::default()
            };

            if blockchain_tree.is_empty() {
                blockchain_tree.set_root(next_node.clone())?;
                self.publish_canonical_update(state, next_node, true, false)?;
            } else if blockchain_tree.has_parent(&next_node) {
                let (height, current_best_block) = blockchain_tree.get_best_tip()?;
                blockchain_tree.add_node(next_node.clone())?;

                match next_node.height.cmp(&height) {
                    std::cmp::Ordering::Equal => {
                        self.process_equal_height(state, &mut blockchain_tree, &current_best_block, next_node).await?;
                    }
                    std::cmp::Ordering::Greater => {
                        self.process_greater_height(state, &mut blockchain_tree, &current_best_block, next_node).await?;
                    }
                    std::cmp::Ordering::Less => {
                        self.publish_canonical_update(state, next_node, false, false)?;
                    }
                }
                blockchain_tree.prune_tree()?;
            }
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_non_canonical_block_with_vrf_info() -> anyhow::Result<()> {
    use crate::stream::{payloads::NewBlockPayload, test_utils::setup_test_actors};
    use ractor::rpc::CallResult;
    use tokio::time::Duration;

    // Setup test actors
    let actor = setup_test_actors(BlockCanonicityActor::new()).await;

    // Create test payloads
    let canonical_block_payload = NewBlockPayload {
        height: 2,
        state_hash: "canonical_hash".to_string(),
        previous_state_hash: "genesis_hash".to_string(),
        last_vrf_output: "b_vrf_output".to_string(),
    };

    let non_canonical_block_payload = NewBlockPayload {
        height: 2,
        state_hash: "non_canonical_hash".to_string(),
        previous_state_hash: "genesis_hash".to_string(),
        last_vrf_output: "a_vrf_output".to_string(),
    };

    // Send test messages and verify responses
    let response = actor
        .call(|_| Event::NewBlock(canonical_block_payload), Some(Duration::from_secs(1)))
        .await?;

    match response {
        CallResult::Success(Event::BlockCanonicityUpdate(payload)) => {
            assert_eq!(payload.height, 2);
            assert_eq!(payload.state_hash, "canonical_hash");
            assert!(payload.canonical);
        }
        CallResult::Success(_) => panic!("Received unexpected message type"),
        CallResult::Timeout => panic!("Call timed out"),
        CallResult::SenderError => panic!("Call failed"),
    }

    let response = actor
        .call(|_| Event::NewBlock(non_canonical_block_payload), Some(Duration::from_secs(1)))
        .await?;

    match response {
        CallResult::Success(Event::BlockCanonicityUpdate(payload)) => {
            assert_eq!(payload.height, 2);
            assert_eq!(payload.state_hash, "non_canonical_hash");
            assert!(!payload.canonical);
        }
        CallResult::Success(_) => panic!("Received unexpected message type"),
        CallResult::Timeout => panic!("Call timed out"),
        CallResult::SenderError => panic!("Call failed"),
    }

    Ok(())
}

#[tokio::test]
async fn test_new_block_becomes_canonical_over_existing_block() -> anyhow::Result<()> {
    use crate::stream::{payloads::NewBlockPayload, test_utils::setup_test_actors};
    use ractor::rpc::CallResult;
    use tokio::time::Duration;

    // Setup test actors
    let actor = setup_test_actors(BlockCanonicityActor::new()).await;

    // Create test payloads
    let initial_block_payload = NewBlockPayload {
        height: 2,
        state_hash: "initial_block_hash".to_string(),
        previous_state_hash: "genesis_hash".to_string(),
        last_vrf_output: "a_vrf_output".to_string(),
    };

    let new_canonical_block_payload = NewBlockPayload {
        height: 2,
        state_hash: "new_canonical_hash".to_string(),
        previous_state_hash: "genesis_hash".to_string(),
        last_vrf_output: "b_vrf_output".to_string(),
    };

    // Send test messages and verify responses
    let response = actor
        .call(|_| Event::NewBlock(initial_block_payload), Some(Duration::from_secs(1)))
        .await?;

    match response {
        CallResult::Success(Event::BlockCanonicityUpdate(payload)) => {
            assert_eq!(payload.height, 2);
            assert_eq!(payload.state_hash, "initial_block_hash");
            assert!(payload.canonical);
        }
        CallResult::Success(_) => panic!("Received unexpected message type"),
        CallResult::Timeout => panic!("Call timed out"),
        CallResult::SenderError => panic!("Call failed"),
    }

    let response = actor
        .call(|_| Event::NewBlock(new_canonical_block_payload), Some(Duration::from_secs(1)))
        .await?;

    match response {
        CallResult::Success(Event::BlockCanonicityUpdate(payload)) => {
            assert_eq!(payload.height, 2);
            assert_eq!(payload.state_hash, "new_canonical_hash");
            assert!(payload.canonical);
        }
        CallResult::Success(_) => panic!("Received unexpected message type"),
        CallResult::Timeout => panic!("Call timed out"),
        CallResult::SenderError => panic!("Call failed"),
    }

    Ok(())
}
