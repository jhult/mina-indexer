use crate::event_sourcing::events::Event;
use futures::lock::Mutex;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::Arc;

#[derive(Clone)]
pub struct BestBlock {
    pub height: u64,
    pub state_hash: String,
}

pub struct BestBlockActor {
    best_block: Arc<Mutex<Option<BestBlock>>>,
}

impl Default for BestBlockActor {
    fn default() -> Self {
        Self {
            best_block: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait::async_trait]
impl Actor for BestBlockActor {
    type Msg = Event;
    type State = ActorRef<Event>;
    type Arguments = ActorRef<Event>;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, parent: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        Ok(parent)
    }

    async fn handle(&self, _myself: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        if let Event::BlockCanonicityUpdate(block_payload) = msg {
            let mut best_block_lock = self.best_block.lock().await;
            match &mut *best_block_lock {
                // Initialize best block if not set
                None => {
                    *best_block_lock = Some(BestBlock {
                        height: block_payload.height,
                        state_hash: block_payload.state_hash.clone(),
                    });
                    state.cast(Event::BestBlock(block_payload))?;
                }
                // Update best block if the new block is canonical and has a higher height
                Some(best_block) if block_payload.canonical && block_payload.height >= best_block.height => {
                    best_block.height = block_payload.height;
                    best_block.state_hash = block_payload.state_hash.clone();
                    state.cast(Event::BestBlock(block_payload))?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_best_block_actor_updates() -> anyhow::Result<()> {
    use crate::event_sourcing::{payloads::BlockCanonicityUpdatePayload, test_utils::setup_test_actors};
    use ractor::rpc::CallResult;
    use tokio::time::Duration;

    // Setup test actors
    let actor = setup_test_actors(BestBlockActor::default()).await;

    // Define a canonical block update payload
    let canonical_block_payload = BlockCanonicityUpdatePayload {
        height: 2,
        state_hash: "new_canonical_hash".to_string(),
        canonical: true,
        was_canonical: false,
    };

    // Send test message and store response
    let response = actor
        .call(|_| Event::BlockCanonicityUpdate(canonical_block_payload.clone()), Some(Duration::from_secs(1)))
        .await?;

    // Verify the response
    match response {
        CallResult::Success(Event::BestBlock(payload)) => {
            assert_eq!(payload.height, 2);
            assert_eq!(payload.state_hash, "new_canonical_hash");
            assert!(payload.canonical);
            assert!(!payload.was_canonical);
        }
        CallResult::Success(_) => panic!("Received unexpected message type"),
        CallResult::Timeout => panic!("Call timed out"),
        CallResult::SenderError => panic!("Call failed"),
    }

    Ok(())
}

#[tokio::test]
async fn test_best_block_actor_does_not_publish_on_non_canonical_or_lower_height_block() -> anyhow::Result<()> {
    use crate::event_sourcing::{payloads::BlockCanonicityUpdatePayload, test_utils::setup_test_actors};
    use ractor::rpc::CallResult;
    use tokio::time::Duration;

    // Setup test actors
    let actor = setup_test_actors(BestBlockActor::default()).await;

    // Initial canonical block
    let initial_canonical_block_payload = BlockCanonicityUpdatePayload {
        height: 2,
        state_hash: "canonical_hash_2".to_string(),
        canonical: true,
        was_canonical: false,
    };

    // Set initial best block
    let response = actor
        .call(
            |_| Event::BlockCanonicityUpdate(initial_canonical_block_payload.clone()),
            Some(Duration::from_secs(1)),
        )
        .await?;

    // Verify initial block was set correctly
    match response {
        CallResult::Success(Event::BestBlock(payload)) => {
            assert_eq!(payload.height, 2);
            assert_eq!(payload.state_hash, "canonical_hash_2");
            assert!(payload.canonical);
        }
        CallResult::Success(_) => panic!("Received unexpected message type"),
        CallResult::Timeout => panic!("Call timed out"),
        CallResult::SenderError => panic!("Call failed"),
    }

    // Test non-canonical block with higher height
    let non_canonical_block_payload = BlockCanonicityUpdatePayload {
        height: 3,
        state_hash: "non_canonical_hash_3".to_string(),
        canonical: false,
        was_canonical: false,
    };

    let response = actor
        .call(|_| Event::BlockCanonicityUpdate(non_canonical_block_payload), Some(Duration::from_secs(1)))
        .await?;

    // Should not receive a BestBlock event for non-canonical block
    assert!(matches!(response, CallResult::Success(Event::BlockCanonicityUpdate(_))));

    // Test canonical block with lower height
    let lower_height_canonical_block_payload = BlockCanonicityUpdatePayload {
        height: 1,
        state_hash: "canonical_hash_1".to_string(),
        canonical: true,
        was_canonical: false,
    };

    let response = actor
        .call(
            |_| Event::BlockCanonicityUpdate(lower_height_canonical_block_payload),
            Some(Duration::from_secs(1)),
        )
        .await?;

    // Should not receive a BestBlock event for lower height block
    assert!(matches!(response, CallResult::Success(Event::BlockCanonicityUpdate(_))));

    Ok(())
}
