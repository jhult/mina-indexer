use super::super::Actor;
use crate::stream::{
    events::Event,
    payloads::{MainnetBlockPayload, SnarkWorkSummaryPayload},
};
use ractor::{ActorProcessingErr, ActorRef};

pub struct SnarkWorkSummaryActor;

#[async_trait::async_trait]
impl Actor for SnarkWorkSummaryActor {
    type Msg = Event;
    type State = ActorRef<Event>;
    type Arguments = ActorRef<Event>;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, parent: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        Ok(parent)
    }

    async fn handle(&self, myself: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match msg {
            Event::MainnetBlock(block_payload) => {
                for snark_job in block_payload.snark_work.iter() {
                    let payload = SnarkWorkSummaryPayload {
                        height: block_payload.height,
                        state_hash: block_payload.state_hash.to_string(),
                        timestamp: block_payload.timestamp,
                        prover: snark_job.prover.to_string(),
                        fee_nanomina: snark_job.fee_nanomina,
                    };
                    myself.cast(Event::SnarkWorkSummary(payload));
                }
            }
            Event::BerkeleyBlock(block_payload) => {
                todo!("impl for berkeley block");
            }
            _ => {}
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_snark_work_summary_actor_with_multiple_snarks() -> anyhow::Result<()> {
    use crate::stream::{mainnet_block_models::MainnetBlock, payloads::MainnetBlockPayload};
    use std::{fs, path::PathBuf};

    async fn verify_snark_work_events(receiver: &mut tokio::sync::broadcast::Receiver<Event>, block_payload: &MainnetBlockPayload, counter: &mut usize) {
        while let Ok(event) = receiver.try_recv() {
            if event.event_type == Event::SnarkWorkSummary {
                let published_payload: SnarkWorkSummaryPayload = sonic_rs::from_str(&event.payload).unwrap();

                // Validate that each event matches the expected block height, state hash, and timestamp
                assert_eq!(published_payload.height, block_payload.height);
                assert_eq!(published_payload.state_hash, block_payload.state_hash);
                assert_eq!(published_payload.timestamp, block_payload.timestamp);

                // Ensure that the published prover and fee are from the expected snark job
                let expected_snark = &block_payload.snark_work[*counter];
                assert_eq!(published_payload.prover, expected_snark.prover);
                assert_eq!(published_payload.fee_nanomina, expected_snark.fee_nanomina);

                *counter += 1;
            }
        }
    }

    // Path to the sample file with 64 snark works
    let path = PathBuf::from("./src/stream/test_data/misc_blocks/mainnet-185-3NKQ3K2SNp58PEAb8UjpBe5uo3KQKxphURuE9Eq2J8JYBVCD7PSu.json");

    // Load and parse the JSON file to simulate the event payload
    let file_content = fs::read_to_string(&path).expect("Could not read test data file");
    let block: MainnetBlock = sonic_rs::from_str(&file_content).expect("Invalid JSON format in test data");
    let block_payload = MainnetBlockPayload {
        height: 185,
        global_slot: 300,
        state_hash: "3NKQ3K2SNp58PEAb8UjpBe5uo3KQKxphURuE9Eq2J8JYBVCD7PSu".to_string(),
        previous_state_hash: block.get_previous_state_hash(),
        last_vrf_output: block.get_last_vrf_output(),
        user_command_count: block.get_user_commands_count(),
        snark_work_count: block.get_aggregated_snark_work().len(),
        snark_work: block.get_aggregated_snark_work(),
        timestamp: block.get_timestamp(),
        coinbase_receiver: block.get_coinbase_receiver(),
        coinbase_reward_nanomina: block.get_coinbase_reward_nanomina(),
        global_slot_since_genesis: block.get_global_slot_since_genesis(),
        user_commands: block.get_user_commands(),
        fee_transfer_via_coinbase: block.get_fee_transfers_via_coinbase(),
        fee_transfers: block.get_fee_transfers(),
        internal_command_count: block.get_internal_command_count(),
        excess_block_fees: block.get_excess_block_fees(),
    };

    // Create shared publisher and SnarkWorkSummaryActor
    let shared_publisher = Arc::new(SharedPublisher::new(1000));
    let actor = SnarkWorkSummaryActor::new(Arc::clone(&shared_publisher));
    let mut receiver = shared_publisher.subscribe();

    // Serialize MainnetBlockPayload to JSON for the event payload
    let payload_json = sonic_rs::to_string(&block_payload).unwrap();
    let event = Event {
        event_type: Event::MainnetBlock,
        payload: payload_json,
    };

    // Call handle_event to process the MainnetBlock event with multiple snark works
    actor.handle_event(event).await;

    // Verify that 64 SnarkWorkSummary events are published
    let mut snark_work_events_received = 0;
    verify_snark_work_events(&mut receiver, &block_payload, &mut snark_work_events_received).await;

    // Ensure that 64 SnarkWorkSummaryPayload events were published
    assert_eq!(snark_work_events_received, 1, "Expected 1 SnarkWorkSummary events");

    Ok(())
}
