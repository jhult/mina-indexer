use super::super::events::Event;
use crate::utility::get_top_level_keys_from_json_file;
use ractor::{Actor, ActorProcessingErr, ActorRef};

#[derive(Default)]
pub struct PCBBlockPathActor;

#[async_trait::async_trait]
impl Actor for PCBBlockPathActor {
    type Msg = Event;
    type State = ActorRef<Event>;
    type Arguments = ActorRef<Event>;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, parent: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        Ok(parent)
    }

    async fn handle(&self, _myself: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        if let Event::PrecomputedBlockPath(path) = msg {
            let keys = get_top_level_keys_from_json_file(&path).expect("file to exist");
            if keys == vec!["data".to_string(), "version".to_string()] {
                state.cast(Event::BerkeleyBlockPath(path))?;
            } else {
                state.cast(Event::MainnetBlockPath(path))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_sourcing::test_utils::setup_test_actors;
    use ractor::rpc::CallResult;
    use std::fs;
    use tempfile::NamedTempFile;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_precomputed_block_path_identity_actor() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(PCBBlockPathActor {}).await;

        // Scenario 1: File with "data" and "version" keys (should trigger BerkeleyBlockPath)
        let temp_file_berkeley = NamedTempFile::new()?;
        fs::write(temp_file_berkeley.path(), r#"{"data": {}, "version": "1.0"}"#)?;
        let berkeley_path = temp_file_berkeley.path().to_str().unwrap().to_string();

        let response = actor
            .call(|_| Event::PrecomputedBlockPath(berkeley_path.clone()), Some(Duration::from_secs(1)))
            .await?;

        match response {
            CallResult::Success(Event::BerkeleyBlockPath(path)) => {
                assert_eq!(path, berkeley_path);
            }
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        // Scenario 2: File with different keys (should trigger MainnetBlockPath)
        let temp_file_mainnet = NamedTempFile::new()?;
        fs::write(temp_file_mainnet.path(), r#"{"other_key": {}, "another_key": "1.0"}"#)?;
        let mainnet_path = temp_file_mainnet.path().to_str().unwrap().to_string();

        let response = actor
            .call(|_| Event::PrecomputedBlockPath(mainnet_path.clone()), Some(Duration::from_secs(1)))
            .await?;

        match response {
            CallResult::Success(Event::MainnetBlockPath(path)) => {
                assert_eq!(path, mainnet_path);
            }
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        Ok(())
    }
}
