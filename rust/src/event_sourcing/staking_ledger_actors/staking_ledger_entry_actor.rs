use super::super::events::Event;
use crate::{
    event_sourcing::{
        payloads::StakingLedgerEntryPayload,
        staking_ledger_models::{StakingEntry, StakingLedger},
    },
    utility::extract_height_and_hash,
};
use async_trait::async_trait;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::path::Path;

#[derive(Default)]
pub struct StakingLedgerEntryActor;

#[async_trait]
impl Actor for StakingLedgerEntryActor {
    type Msg = Event;
    type State = ActorRef<Event>;
    type Arguments = ActorRef<Event>;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, parent: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        Ok(parent)
    }

    async fn handle(&self, _myself: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        if let Event::StakingLedgerFilePath(file_path) = msg {
            let file_content =
                std::fs::read_to_string(&file_path).map_err(|e| ActorProcessingErr::from(format!("Failed to read staking ledger file: {}", e)))?;
            let staking_ledger_entries: Vec<StakingEntry> =
                sonic_rs::from_str(&file_content).map_err(|e| ActorProcessingErr::from(format!("Failed to parse staking ledger JSON: {}", e)))?;

            let filename = Path::new(&file_path);
            let (epoch, _) = extract_height_and_hash(filename);

            let staking_ledger = StakingLedger::new(staking_ledger_entries, epoch as u64);

            let stakes_map = staking_ledger.get_stakes(staking_ledger.get_total_staked());

            for (_, stake) in stakes_map.iter() {
                let payload = StakingLedgerEntryPayload {
                    epoch: stake.epoch,
                    delegate: stake.delegate.to_string(),
                    stake: stake.stake,
                    total_staked: stake.total_staked,
                    delegators_count: stake.delegators.len() as u64,
                };
                state.cast(Event::StakingLedgerEntry(payload))?;
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
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_staking_ledger_entry_actor() -> anyhow::Result<()> {
        // Prepare test file path
        let file_path = "./src/event_sourcing/test_data/staking_ledgers/mainnet-9-jxVLvFcBbRCDSM8MHLam6UPVPo2KDegbzJN6MTZWyhTvDrPcjYk.json";

        // Setup test actors
        let actor = setup_test_actors(StakingLedgerEntryActor::default()).await;

        // Send test message and store response
        let response = actor
            .call(|_| Event::StakingLedgerFilePath(file_path.to_string()), Some(Duration::from_secs(1)))
            .await?;

        // Verify the response
        match response {
            CallResult::Success(Event::StakingLedgerEntry(payload)) => {
                // TODO: add more assertions based on expected payload values
                assert_eq!(payload.epoch, 9);
            }
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        Ok(())
    }
}
