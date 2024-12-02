use crate::event_sourcing::{events::Event, payloads::BlockLogPayload};
use ractor::{Actor, ActorProcessingErr, ActorRef};

#[derive(Default)]
pub struct BlockLogActor;

#[async_trait::async_trait]
impl Actor for BlockLogActor {
    type Msg = Event;
    type State = ActorRef<Event>;
    type Arguments = ActorRef<Event>;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, parent: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        Ok(parent)
    }

    async fn handle(&self, _myself: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match msg {
            Event::GenesisBlock(block_payload) => {
                let payload = BlockLogPayload {
                    height: block_payload.height,
                    state_hash: block_payload.state_hash,
                    previous_state_hash: block_payload.previous_state_hash,
                    user_command_count: 0,
                    snark_work_count: 0,
                    timestamp: block_payload.unix_timestamp,
                    coinbase_receiver: String::new(),
                    coinbase_reward_nanomina: 0,
                    global_slot_since_genesis: 1,
                    last_vrf_output: block_payload.last_vrf_output,
                    is_berkeley_block: false,
                };
                state.cast(Event::BlockLog(payload))?;
            }
            Event::MainnetBlock(block_payload) => {
                let payload = BlockLogPayload {
                    height: block_payload.height,
                    state_hash: block_payload.state_hash,
                    previous_state_hash: block_payload.previous_state_hash,
                    user_command_count: block_payload.user_command_count,
                    snark_work_count: block_payload.snark_work_count,
                    timestamp: block_payload.timestamp,
                    coinbase_receiver: block_payload.coinbase_receiver,
                    coinbase_reward_nanomina: block_payload.coinbase_reward_nanomina,
                    global_slot_since_genesis: block_payload.global_slot_since_genesis,
                    last_vrf_output: block_payload.last_vrf_output,
                    is_berkeley_block: false,
                };
                state.cast(Event::BlockLog(payload))?;
            }
            Event::BerkeleyBlock(_) => {
                todo!("impl for berkeley block");
            }
            _ => {}
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_block_log_actor_with_mainnet_block() -> anyhow::Result<()> {
    use crate::event_sourcing::{payloads::MainnetBlockPayload, test_utils::setup_test_actors};
    use ractor::rpc::CallResult;
    use tokio::time::Duration;

    // Setup test actors
    let actor = setup_test_actors(BlockLogActor {}).await;

    // Create test payload
    let block_payload = MainnetBlockPayload {
        height: 100,
        last_vrf_output: "some_vrf_output".to_string(),
        state_hash: "some_state_hash".to_string(),
        previous_state_hash: "previous_state_hash".to_string(),
        user_command_count: 5,
        snark_work_count: 3,
        snark_work: vec![],
        timestamp: 1623423000,
        coinbase_receiver: "receiver_public_key".to_string(),
        coinbase_reward_nanomina: 720_000_000_000,
        global_slot_since_genesis: 12345,
        ..Default::default()
    };

    // Send test message and store response
    let response = actor.call(|_| Event::MainnetBlock(block_payload), Some(Duration::from_secs(1))).await?;

    // Verify the response
    match response {
        CallResult::Success(Event::BlockLog(payload)) => {
            assert_eq!(payload.height, 100);
            assert_eq!(payload.state_hash, "some_state_hash");
            assert_eq!(payload.previous_state_hash, "previous_state_hash");
            assert_eq!(payload.user_command_count, 5);
            assert_eq!(payload.snark_work_count, 3);
            assert_eq!(payload.timestamp, 1623423000);
            assert_eq!(payload.coinbase_receiver, "receiver_public_key");
            assert_eq!(payload.coinbase_reward_nanomina, 720_000_000_000);
            assert_eq!(payload.global_slot_since_genesis, 12345);
            assert!(!payload.is_berkeley_block);
        }
        CallResult::Success(_) => panic!("Received unexpected message type"),
        CallResult::Timeout => panic!("Call timed out"),
        CallResult::SenderError => panic!("Call failed"),
    }

    Ok(())
}

#[tokio::test]
async fn test_block_log_actor_with_genesis_block() -> anyhow::Result<()> {
    use crate::event_sourcing::{payloads::GenesisBlockPayload, test_utils::setup_test_actors};
    use ractor::rpc::CallResult;
    use tokio::time::Duration;

    // Setup test actors
    let actor = setup_test_actors(BlockLogActor {}).await;

    // Create test payload using the default genesis block
    let genesis_block = GenesisBlockPayload::default();

    // Send test message and store response
    let response = actor.call(|_| Event::GenesisBlock(genesis_block), Some(Duration::from_secs(1))).await?;

    // Verify the response
    match response {
        CallResult::Success(Event::BlockLog(payload)) => {
            assert_eq!(payload.height, 1);
            assert_eq!(payload.state_hash, "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ");
            assert_eq!(payload.previous_state_hash, "3NLoKn22eMnyQ7rxh5pxB6vBA3XhSAhhrf7akdqS6HbAKD14Dh1d");
            assert_eq!(payload.user_command_count, 0);
            assert_eq!(payload.snark_work_count, 0);
            assert_eq!(payload.timestamp, 1615939200000);
            assert_eq!(payload.coinbase_receiver, "");
            assert_eq!(payload.coinbase_reward_nanomina, 0);
            assert_eq!(payload.global_slot_since_genesis, 1);
            assert!(!payload.is_berkeley_block);
        }
        CallResult::Success(_) => panic!("Received unexpected message type"),
        CallResult::Timeout => panic!("Call timed out"),
        CallResult::SenderError => panic!("Call failed"),
    }

    Ok(())
}
