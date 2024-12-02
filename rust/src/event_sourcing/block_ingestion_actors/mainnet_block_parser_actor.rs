use super::super::events::Event;
use crate::{
    event_sourcing::{error_unhandled_event, mainnet_block_models::MainnetBlock, payloads::MainnetBlockPayload},
    utility::extract_height_and_hash,
};
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::{fs, path::Path};

#[derive(Default)]
pub struct MainnetBlockParserActor;

#[async_trait::async_trait]
impl Actor for MainnetBlockParserActor {
    type Msg = Event;
    type State = ActorRef<Event>;
    type Arguments = ActorRef<Event>;

    async fn pre_start(&self, _: ActorRef<Self::Msg>, parent: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        Ok(parent)
    }

    async fn handle(&self, _: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match msg {
            Event::MainnetBlockPath(path) => {
                let path = Path::new(&path);
                let (height, state_hash) = extract_height_and_hash(path);
                let file_content = fs::read_to_string(path).expect("Failed to read JSON file from disk");
                let block: MainnetBlock = sonic_rs::from_str(&file_content).unwrap();
                let block_payload = MainnetBlockPayload {
                    height: height as u64,
                    global_slot: block.get_global_slot_since_genesis(),
                    state_hash: state_hash.to_string(),
                    previous_state_hash: block.get_previous_state_hash(),
                    last_vrf_output: block.get_last_vrf_output(),
                    user_command_count: block.get_user_commands_count(),
                    snark_work_count: block.get_aggregated_snark_work().len(),
                    snark_work: block.get_aggregated_snark_work(),
                    timestamp: block.get_timestamp(),
                    coinbase_reward_nanomina: block.get_coinbase_reward_nanomina(),
                    coinbase_receiver: block.get_coinbase_receiver(),
                    global_slot_since_genesis: block.get_global_slot_since_genesis(),
                    user_commands: block.get_user_commands(),
                    fee_transfer_via_coinbase: block.get_fee_transfers_via_coinbase(),
                    fee_transfers: block.get_fee_transfers(),
                    internal_command_count: block.get_internal_command_count(),
                    excess_block_fees: block.get_excess_block_fees(),
                };
                state.cast(Event::MainnetBlock(block_payload));
                Ok(())
            }
            _ => error_unhandled_event(&msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_sourcing::test_utils::setup_test_actors;
    use ractor::rpc::CallResult;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_mainnet_block_parser_actor() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(MainnetBlockParserActor::default()).await;

        // Define paths for two block files
        let block_file_100 = "./src/event_sourcing/test_data/100_mainnet_blocks/mainnet-100-3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4.json";
        let block_file_99 = "./src/event_sourcing/test_data/100_mainnet_blocks/mainnet-99-3NLdywCHZmuqxS4hUnW7Uuu6sr97iifh5Ldc6m9EbzVZyLqbxqCh.json";

        // Test block 100
        let response_100 = actor
            .call(|_| Event::MainnetBlockPath(block_file_100.to_string()), Some(Duration::from_secs(1)))
            .await?;

        // Verify the response for block 100
        match response_100 {
            CallResult::Success(Event::MainnetBlock(payload)) => {
                assert_eq!(payload.height, 100);
                assert_eq!(payload.state_hash, "3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4");
                assert_eq!(payload.previous_state_hash, "3NLdywCHZmuqxS4hUnW7Uuu6sr97iifh5Ldc6m9EbzVZyLqbxqCh");
                assert_eq!(payload.last_vrf_output, "HXzRY01h73mWXp4cjNwdDTYLDtdFU5mYhTbWWi-1wwE=");
                assert_eq!(payload.user_command_count, 1);
                assert_eq!(payload.snark_work_count, 0);
            }
            CallResult::Success(_) => panic!("Received unexpected message type for block 100"),
            CallResult::Timeout => panic!("Call timed out for block 100"),
            CallResult::SenderError => panic!("Call failed for block 100"),
        }

        // Test block 99
        let response_99 = actor
            .call(|_| Event::MainnetBlockPath(block_file_99.to_string()), Some(Duration::from_secs(1)))
            .await?;

        // Verify the response for block 99
        match response_99 {
            CallResult::Success(Event::MainnetBlock(payload)) => {
                assert_eq!(payload.height, 99);
                assert_eq!(payload.state_hash, "3NLdywCHZmuqxS4hUnW7Uuu6sr97iifh5Ldc6m9EbzVZyLqbxqCh");
                assert_eq!(payload.previous_state_hash, "3NLAuBJPgT4Tk4LpufAEDQq4Jv9QVUefq3n3eB9x9VgGqe6LKzWp");
                assert_eq!(payload.last_vrf_output, "ws1xspEgjEyLiSS0V2-Egf9UzJG3FACpruvvDEsqDAA=");
                assert_eq!(payload.user_command_count, 3);
                assert_eq!(payload.snark_work_count, 0);
            }
            CallResult::Success(_) => panic!("Received unexpected message type for block 99"),
            CallResult::Timeout => panic!("Call timed out for block 99"),
            CallResult::SenderError => panic!("Call failed for block 99"),
        }

        Ok(())
    }
}
