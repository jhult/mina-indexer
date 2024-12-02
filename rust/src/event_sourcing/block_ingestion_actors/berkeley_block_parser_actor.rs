use super::super::events::Event;
use crate::{
    event_sourcing::{berkeley_block_models::BerkeleyBlock, payloads::BerkeleyBlockPayload},
    utility::extract_height_and_hash,
};
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::{fs, path::Path};

pub struct BerkeleyBlockParserActor;

impl Default for BerkeleyBlockParserActor {
    fn default() -> Self {
        BerkeleyBlockParserActor {}
    }
}

#[async_trait::async_trait]
impl Actor for BerkeleyBlockParserActor {
    type Msg = Event;
    type State = ActorRef<Event>;
    type Arguments = ActorRef<Event>;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, parent: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        Ok(parent)
    }

    async fn handle(&self, _myself: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        if let Event::BerkeleyBlockPath(path_str) = msg {
            let path = Path::new(&path_str);
            let (height, state_hash) = extract_height_and_hash(path);

            let file_content = fs::read_to_string(path).map_err(|e| ActorProcessingErr::from(format!("Failed to read JSON file: {}", e)))?;

            let berkeley_block: BerkeleyBlock =
                sonic_rs::from_str(&file_content).map_err(|e| ActorProcessingErr::from(format!("Failed to parse Berkeley block: {}", e)))?;

            let berkeley_block_payload = BerkeleyBlockPayload {
                height: height as u64,
                state_hash: state_hash.to_string(),
                previous_state_hash: berkeley_block.get_previous_state_hash(),
                last_vrf_output: berkeley_block.get_last_vrf_output(),
            };

            state.cast(Event::BerkeleyBlock(berkeley_block_payload))?;
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_berkeley_block_parser_actor() -> anyhow::Result<()> {
    use crate::event_sourcing::test_utils::setup_test_actors;
    use ractor::rpc::CallResult;
    use tokio::time::Duration;

    // Setup test actors
    let actor = setup_test_actors(BerkeleyBlockParserActor {}).await;

    let block_file = "./src/stream/test_data/berkeley_blocks/berkeley-10-3NL53c8uTVnoFjzh17VAeCR9r3zjmDowNpeFRRVEUqnvX5WdHtWE.json";

    // Send test message and store response
    let response = actor
        .call(|_| Event::BerkeleyBlockPath(block_file.to_string()), Some(Duration::from_secs(1)))
        .await?;

    // Verify the response
    match response {
        CallResult::Success(Event::BerkeleyBlock(payload)) => {
            assert_eq!(payload.height, 10);
            assert_eq!(payload.state_hash, "3NL53c8uTVnoFjzh17VAeCR9r3zjmDowNpeFRRVEUqnvX5WdHtWE");
            assert_eq!(payload.previous_state_hash, "3NKJarZEsMAHkcPfhGA72eyjWBXGHergBZEoTuGXWS7vWeq8D5wu");
            assert_eq!(payload.last_vrf_output, "hu0nffAHwdL0CYQNAlabyiUlwNWhlbj0MwynpKLtAAA=");
        }
        CallResult::Success(_) => panic!("Received unexpected message type"),
        CallResult::Timeout => panic!("Call timed out"),
        CallResult::SenderError => panic!("Call failed"),
    }

    Ok(())
}
