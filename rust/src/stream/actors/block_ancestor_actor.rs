use super::super::events::Event;
use crate::stream::payloads::BlockAncestorPayload;
use ractor::{Actor, ActorProcessingErr, ActorRef};

pub struct BlockAncestorActor;

#[async_trait::async_trait]
impl Actor for BlockAncestorActor {
    type Msg = Event;
    type State = ActorRef<Event>;
    type Arguments = ActorRef<Event>;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, parent: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        Ok(parent)
    }

    async fn handle(&self, _myself: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match msg {
            Event::BerkeleyBlock(block_payload) => {
                let block_ancestor_payload = BlockAncestorPayload {
                    height: block_payload.height,
                    state_hash: block_payload.state_hash,
                    previous_state_hash: block_payload.previous_state_hash,
                    last_vrf_output: block_payload.last_vrf_output,
                };
                state.cast(Event::BlockAncestor(block_ancestor_payload))?;
            }
            Event::MainnetBlock(block_payload) => {
                let block_ancestor_payload = BlockAncestorPayload {
                    height: block_payload.height,
                    state_hash: block_payload.state_hash,
                    previous_state_hash: block_payload.previous_state_hash,
                    last_vrf_output: block_payload.last_vrf_output,
                };
                state.cast(Event::BlockAncestor(block_ancestor_payload))?;
            }
            _ => {}
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_block_ancestor_actor_with_berkeley_block() -> anyhow::Result<()> {
    use crate::stream::{payloads::BerkeleyBlockPayload, test_utils::setup_test_actors};
    use ractor::rpc::CallResult;
    use tokio::time::Duration;

    // Setup test actors
    let actor = setup_test_actors(BlockAncestorActor {}).await;

    // Create test payload
    let berkeley_block_payload = BerkeleyBlockPayload {
        height: 89,
        state_hash: "3NKVkEwELHY9CmPYxf25pwsKZpPf161QVCiC3JwdsyQwCYyE3wNCrRjWON".to_string(),
        previous_state_hash: "3NKJarZEsMAHkcPfhGA72eyjWBXGHergBZEoTuGXWS7vWeq8D5wu".to_string(),
        last_vrf_output: "hu0nffAHwdL0CYQNAlabyiUlwNWhlbj0MwynpKLtAAA=".to_string(),
    };

    // Send test message and store response
    let response = actor
        .call(|_| Event::BerkeleyBlock(berkeley_block_payload), Some(Duration::from_secs(1)))
        .await?;

    // Verify the response
    match response {
        CallResult::Success(Event::BlockAncestor(payload)) => {
            assert_eq!(payload.height, 89);
            assert_eq!(payload.state_hash, "3NKVkEwELHY9CmPYxf25pwsKZpPf161QVCiC3JwdsyQwCYyE3wNCrRjWON");
            assert_eq!(payload.previous_state_hash, "3NKJarZEsMAHkcPfhGA72eyjWBXGHergBZEoTuGXWS7vWeq8D5wu");
            assert_eq!(payload.last_vrf_output, "hu0nffAHwdL0CYQNAlabyiUlwNWhlbj0MwynpKLtAAA=");
        }
        CallResult::Success(_) => panic!("Received unexpected message type"),
        CallResult::Timeout => panic!("Call timed out"),
        CallResult::SenderError => panic!("Call failed"),
    }

    Ok(())
}

#[tokio::test]
async fn test_block_ancestor_actor_with_mainnet_block() -> anyhow::Result<()> {
    use crate::stream::{payloads::MainnetBlockPayload, test_utils::setup_test_actors};
    use ractor::rpc::CallResult;
    use tokio::time::Duration;

    // Setup test actors
    let actor = setup_test_actors(BlockAncestorActor {}).await;

    // Create test payload
    let mainnet_block_payload = MainnetBlockPayload {
        height: 101,
        state_hash: "4MTNpwef32H67dHk9Mx25ZLpHfVz27QXECm8C4o5eyRa5LgJ1qLScCwpJM".to_string(),
        previous_state_hash: "4MPXcYhJY8URpwZxBEmv9C7kXf5h41PLXeX9GoTwFg3TuL2Q9zMn".to_string(),
        last_vrf_output: "WXPOLoGn9vE7HwqkE-K5bH4d3LmSPPJQcfoLsrTDkQA=".to_string(),
        user_command_count: 4,
        snark_work_count: 0,
        timestamp: 1615986540000,
        coinbase_receiver: "B62qjA7LFMvKuzFbGZK9yb3wAkBThba1pe5ap8UZx8jEvfAEcnDgDBE".to_string(),
        coinbase_reward_nanomina: 720_000_000_000,
        global_slot_since_genesis: 148,
        ..Default::default()
    };

    // Send test message and store response
    let response = actor.call(|_| Event::MainnetBlock(mainnet_block_payload), Some(Duration::from_secs(1))).await?;

    // Verify the response
    match response {
        CallResult::Success(Event::BlockAncestor(payload)) => {
            assert_eq!(payload.height, 101);
            assert_eq!(payload.state_hash, "4MTNpwef32H67dHk9Mx25ZLpHfVz27QXECm8C4o5eyRa5LgJ1qLScCwpJM");
            assert_eq!(payload.previous_state_hash, "4MPXcYhJY8URpwZxBEmv9C7kXf5h41PLXeX9GoTwFg3TuL2Q9zMn");
            assert_eq!(payload.last_vrf_output, "WXPOLoGn9vE7HwqkE-K5bH4d3LmSPPJQcfoLsrTDkQA=");
        }
        CallResult::Success(_) => panic!("Received unexpected message type"),
        CallResult::Timeout => panic!("Call timed out"),
        CallResult::SenderError => panic!("Call failed"),
    }

    Ok(())
}
