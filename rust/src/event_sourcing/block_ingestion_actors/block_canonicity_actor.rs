use crate::{
    blockchain_tree::{BlockchainTree, Hash, Height, Node},
    event_sourcing::{events::Event, payloads::BlockCanonicityUpdatePayload},
};
use futures::lock::Mutex;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::Arc;

pub struct BlockCanonicityActor {
    blockchain_tree: Arc<Mutex<BlockchainTree>>,
}

impl Default for BlockCanonicityActor {
    fn default() -> Self {
        Self {
            blockchain_tree: Arc::new(Mutex::new(BlockchainTree::new(11))),
        }
    }
}

impl BlockCanonicityActor {
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
                return Ok(());
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
            } else {
                println!(
                    "Attempted to add block and height {} and state_hash {} but found no parent",
                    next_node.height.0, next_node.state_hash.0
                )
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        constants::GENESIS_STATE_HASH,
        event_sourcing::{payloads::NewBlockPayload, test_utils::setup_test_actors},
    };
    use ractor::rpc::CallResult;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_non_canonical_block_with_vrf_info() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(BlockCanonicityActor::default()).await;

        // Create test payloads
        let canonical_block_payload = NewBlockPayload {
            height: 2,
            state_hash: "canonical_hash".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "b_vrf_output".to_string(),
        };
        let non_canonical_block_payload = NewBlockPayload {
            height: 2,
            state_hash: "non_canonical_hash".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "a_vrf_output".to_string(),
        };

        // Send test messages and verify responses
        let response = actor
            .call(|_| Event::NewBlock(canonical_block_payload.clone()), Some(Duration::from_secs(1)))
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

        let response = actor
            .call(|_| Event::NewBlock(non_canonical_block_payload), Some(Duration::from_secs(1)))
            .await?;

        match response {
            CallResult::Success(Event::BlockCanonicityUpdate(payload)) => {
                assert_eq!(payload.height, 2);
                assert_eq!(payload.state_hash, "non_canonical_hash");
                assert!(!payload.canonical); // Ensure the non-canonical block is marked as non-canonical
            }
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_new_block_becomes_canonical_over_existing_block() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(BlockCanonicityActor::default()).await;

        // Create test payloads
        let initial_block_payload = NewBlockPayload {
            height: 2,
            state_hash: "initial_block_hash".to_string(),
            last_vrf_output: "a_vrf_output".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
        };

        let new_canonical_block_payload = NewBlockPayload {
            height: 2,
            state_hash: "new_canonical_hash".to_string(),
            last_vrf_output: "b_vrf_output".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
        };
        // Send test messages and verify responses
        let response = actor.call(|_| Event::NewBlock(initial_block_payload), Some(Duration::from_secs(1))).await?;

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

    #[tokio::test]
    async fn test_longer_branch_outcompetes_canonical_branch_with_tiebreaker() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(BlockCanonicityActor::default()).await;

        // Create canonical block payloads in the original branch
        let original_block1_payload = NewBlockPayload {
            height: 2,
            state_hash: "original_block_1".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "b_vrf_output".to_string(),
        };
        let original_block2_payload = NewBlockPayload {
            height: 3,
            state_hash: "original_block_2".to_string(),
            previous_state_hash: "original_block_1".to_string(),
            last_vrf_output: "b_vrf_output".to_string(),
        };

        // Create competing branch payloads
        let competing_block1_payload = NewBlockPayload {
            height: 2,
            state_hash: "competing_block_1".to_string(),
            previous_state_hash: "genesis_hash".to_string(),
            last_vrf_output: "a_vrf_output".to_string(),
        };
        let competing_block2_payload = NewBlockPayload {
            height: 3,
            state_hash: "competing_block_2".to_string(),
            previous_state_hash: "competing_block_1".to_string(),
            last_vrf_output: "a_vrf_output".to_string(),
        };
        let competing_block3_payload = NewBlockPayload {
            height: 4,
            state_hash: "competing_block_3".to_string(),
            previous_state_hash: "competing_block_2".to_string(),
            last_vrf_output: "a_vrf_output".to_string(),
        };

        // Send test messages and verify responses
        actor.call(|_| Event::NewBlock(original_block1_payload), Some(Duration::from_secs(1))).await?;
        actor.call(|_| Event::NewBlock(original_block2_payload), Some(Duration::from_secs(1))).await?;
        actor.call(|_| Event::NewBlock(competing_block1_payload), Some(Duration::from_secs(1))).await?;
        actor.call(|_| Event::NewBlock(competing_block2_payload), Some(Duration::from_secs(1))).await?;
        actor.call(|_| Event::NewBlock(competing_block3_payload), Some(Duration::from_secs(1))).await?;

        // Expected sequence of events
        let expected_events = vec![
            // Initially, both original blocks are marked as canonical
            BlockCanonicityUpdatePayload {
                height: 2,
                state_hash: "original_block_1".to_string(),
                canonical: true,
                was_canonical: false,
            },
            BlockCanonicityUpdatePayload {
                height: 3,
                state_hash: "original_block_2".to_string(),
                canonical: true,
                was_canonical: false,
            },
            // Competing blocks are added as non-canonical until the tiebreaker
            BlockCanonicityUpdatePayload {
                height: 2,
                state_hash: "competing_block_1".to_string(),
                canonical: false,
                was_canonical: false,
            },
            BlockCanonicityUpdatePayload {
                height: 3,
                state_hash: "competing_block_2".to_string(),
                canonical: false,
                was_canonical: false,
            },
            // Competing branch wins, update the blocks
            BlockCanonicityUpdatePayload {
                height: 3,
                state_hash: "original_block_2".to_string(),
                canonical: false,
                was_canonical: true,
            },
            BlockCanonicityUpdatePayload {
                height: 2,
                state_hash: "original_block_1".to_string(),
                canonical: false,
                was_canonical: true,
            },
            BlockCanonicityUpdatePayload {
                height: 2,
                state_hash: "competing_block_1".to_string(),
                canonical: true,
                was_canonical: false,
            },
            BlockCanonicityUpdatePayload {
                height: 3,
                state_hash: "competing_block_2".to_string(),
                canonical: true,
                was_canonical: false,
            },
            BlockCanonicityUpdatePayload {
                height: 4,
                state_hash: "competing_block_3".to_string(),
                canonical: true,
                was_canonical: false,
            },
        ];

        // Verify the sequence of events
        for expected_event in expected_events.into_iter() {
            let received_event = actor
                .call(|_| Event::NewBlock(competing_block3_payload.clone()), Some(Duration::from_secs(1)))
                .await?;
            match received_event {
                CallResult::Success(Event::BlockCanonicityUpdate(payload)) => {
                    assert_eq!(payload, expected_event);
                }
                CallResult::Success(_) => panic!("Received unexpected message type"),
                CallResult::Timeout => panic!("Call timed out"),
                CallResult::SenderError => panic!("Call failed"),
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_block_with_different_parent_at_same_height() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(BlockCanonicityActor::default()).await;

        // Root block payload
        let root_block = NewBlockPayload {
            height: 1228,
            state_hash: "3NLGaCf4zNXouyKDkmrFuYBVt8BHnCuLC9PMvkytfFgiuuqv5xsH".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "QNWhrsZ3Ld1gsJTUGBUBCs9iQD00hKUvZkR1GeFjEgA=".to_string(),
        };

        // First branch payloads
        let branch1_block1 = NewBlockPayload {
            height: 1229,
            state_hash: "3NLU6jZai3xytD6PkpdKNFTYjfYSsEWoksr9AmufqYbxJN2Mpuke".to_string(),
            previous_state_hash: root_block.state_hash.clone(),
            last_vrf_output: "PhNdMGQWACdnDnOg3D75icmHf5Mu_0F44ua-fzz4DwA=".to_string(),
        };
        let branch1_block2 = NewBlockPayload {
            height: 1230,
            state_hash: "3NKNqeqRjSY8Qy1x4qwzfCVSCGUq2czp4Jo9E7pUY6ZnHTBg4JiW".to_string(),
            previous_state_hash: branch1_block1.state_hash.clone(),
            last_vrf_output: "h8fO60ijmmBdiMfCBSPz47vsRW8BHg2hnYo4iwl3BAA=".to_string(),
        };
        let branch1_block3 = NewBlockPayload {
            height: 1231,
            state_hash: "3NKMBzGM1pySPanKkLdxjnUS9mZ88oCeanq3Nhm2QZ6eNB5ZkeGn".to_string(),
            previous_state_hash: branch1_block2.state_hash.clone(),
            last_vrf_output: "h8fO60ijmmBdiMfCBSPz47vsRW8BHg2hnYo4iwl3BAA=".to_string(),
        };

        // Second branch payloads
        let branch2_block1 = NewBlockPayload {
            height: 1229,
            state_hash: "3NL63pX2kKi4pezYnm6MPjsDK9VSvJ1XFiUaed9vxPEa5PuWPLeZ".to_string(),
            previous_state_hash: root_block.state_hash.clone(),
            last_vrf_output: "hPuL8ZcZI2pIZhbVLVVD0U4CO0FLlnIgW_rYULV_HAA=".to_string(),
        };
        let branch2_block2 = NewBlockPayload {
            height: 1230,
            state_hash: "3NKcSppgUnmuGp9kGQdp7ZXUh5vcf7rm9mhKE4UMvwcuUaB8wb7S".to_string(),
            previous_state_hash: branch2_block1.state_hash.clone(),
            last_vrf_output: "Wl4EiJDCuzNCCn2w_6vDywRZStf4iNM5h4qUxXvOFAA=".to_string(),
        };
        let branch2_block3 = NewBlockPayload {
            height: 1231,
            state_hash: "3NLN2uxCoRsiB2C4uZHDcY6WxJDz6EEWmoruCFva74E4cgTYgJCQ".to_string(),
            previous_state_hash: branch2_block2.state_hash.clone(),
            last_vrf_output: "qQNTDpNokYfZs1wlQNcSLWazVCu43O7mLAhxlgwjBAA=".to_string(),
        };

        // Initialize with the root block
        actor.call(|_| Event::NewBlock(root_block.clone()), Some(Duration::from_secs(1))).await?;

        // Process branch 1
        actor.call(|_| Event::NewBlock(branch1_block1.clone()), Some(Duration::from_secs(1))).await?;
        actor.call(|_| Event::NewBlock(branch1_block2.clone()), Some(Duration::from_secs(1))).await?;
        actor.call(|_| Event::NewBlock(branch1_block3.clone()), Some(Duration::from_secs(1))).await?;

        // Process branch 2
        actor.call(|_| Event::NewBlock(branch2_block1.clone()), Some(Duration::from_secs(1))).await?;
        actor.call(|_| Event::NewBlock(branch2_block2.clone()), Some(Duration::from_secs(1))).await?;
        actor.call(|_| Event::NewBlock(branch2_block3.clone()), Some(Duration::from_secs(1))).await?;

        let expected_events = vec![
            BlockCanonicityUpdatePayload {
                height: 1231,
                state_hash: "3NLN2uxCoRsiB2C4uZHDcY6WxJDz6EEWmoruCFva74E4cgTYgJCQ".to_string(),
                canonical: true,
                was_canonical: false,
            },
            BlockCanonicityUpdatePayload {
                height: 1230,
                state_hash: "3NKcSppgUnmuGp9kGQdp7ZXUh5vcf7rm9mhKE4UMvwcuUaB8wb7S".to_string(),
                canonical: true,
                was_canonical: false,
            },
            BlockCanonicityUpdatePayload {
                height: 1229,
                state_hash: "3NL63pX2kKi4pezYnm6MPjsDK9VSvJ1XFiUaed9vxPEa5PuWPLeZ".to_string(),
                canonical: true,
                was_canonical: false,
            },
            BlockCanonicityUpdatePayload {
                height: 1229,
                state_hash: "3NLU6jZai3xytD6PkpdKNFTYjfYSsEWoksr9AmufqYbxJN2Mpuke".to_string(),
                canonical: false,
                was_canonical: true,
            },
            BlockCanonicityUpdatePayload {
                height: 1230,
                state_hash: "3NKNqeqRjSY8Qy1x4qwzfCVSCGUq2czp4Jo9E7pUY6ZnHTBg4JiW".to_string(),
                canonical: false,
                was_canonical: true,
            },
            BlockCanonicityUpdatePayload {
                height: 1231,
                state_hash: "3NKMBzGM1pySPanKkLdxjnUS9mZ88oCeanq3Nhm2QZ6eNB5ZkeGn".to_string(),
                canonical: false,
                was_canonical: true,
            },
            BlockCanonicityUpdatePayload {
                height: 1230,
                state_hash: "3NKcSppgUnmuGp9kGQdp7ZXUh5vcf7rm9mhKE4UMvwcuUaB8wb7S".to_string(),
                canonical: false,
                was_canonical: false,
            },
            BlockCanonicityUpdatePayload {
                height: 1229,
                state_hash: "3NL63pX2kKi4pezYnm6MPjsDK9VSvJ1XFiUaed9vxPEa5PuWPLeZ".to_string(),
                canonical: false,
                was_canonical: false,
            },
            BlockCanonicityUpdatePayload {
                height: 1231,
                state_hash: "3NKMBzGM1pySPanKkLdxjnUS9mZ88oCeanq3Nhm2QZ6eNB5ZkeGn".to_string(),
                canonical: true,
                was_canonical: false,
            },
            BlockCanonicityUpdatePayload {
                height: 1230,
                state_hash: "3NKNqeqRjSY8Qy1x4qwzfCVSCGUq2czp4Jo9E7pUY6ZnHTBg4JiW".to_string(),
                canonical: true,
                was_canonical: false,
            },
            BlockCanonicityUpdatePayload {
                height: 1229,
                state_hash: "3NLU6jZai3xytD6PkpdKNFTYjfYSsEWoksr9AmufqYbxJN2Mpuke".to_string(),
                canonical: true,
                was_canonical: false,
            },
        ];

        // Verify the sequence of events
        for expected_event in expected_events.into_iter() {
            let received_event = actor.call(|_| Event::NewBlock(branch2_block3.clone()), Some(Duration::from_secs(1))).await?;
            match received_event {
                CallResult::Success(Event::BlockCanonicityUpdate(payload)) => {
                    assert_eq!(payload, expected_event);
                }
                CallResult::Success(_) => panic!("Received unexpected message type"),
                CallResult::Timeout => panic!("Call timed out"),
                CallResult::SenderError => panic!("Call failed"),
            }
        }
        Ok(())
    }
}
