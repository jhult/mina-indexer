use crate::{
    constants::TRANSITION_FRONTIER_DISTANCE,
    event_sourcing::{canonical_items_manager::CanonicalItemsManager, events::Event, payloads::*},
};
use futures::lock::Mutex;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::Arc;

pub struct SnarkCanonicitySummaryActor {
    canonical_items_manager: Arc<Mutex<CanonicalItemsManager<SnarkCanonicitySummaryPayload>>>,
}

impl Default for SnarkCanonicitySummaryActor {
    fn default() -> Self {
        Self {
            canonical_items_manager: Arc::new(Mutex::new(CanonicalItemsManager::new((TRANSITION_FRONTIER_DISTANCE / 5usize) as u64))),
        }
    }
}

#[async_trait::async_trait]
impl Actor for SnarkCanonicitySummaryActor {
    type Msg = Event;
    type State = ActorRef<Event>;
    type Arguments = ActorRef<Event>;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, parent: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        Ok(parent)
    }

    async fn handle(&self, _myself: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match msg {
            Event::BlockCanonicityUpdate(payload) => {
                {
                    let manager = self.canonical_items_manager.lock().await;
                    manager.add_block_canonicity_update(payload.clone()).await;
                }
                {
                    let manager = self.canonical_items_manager.lock().await;
                    for update_payload in manager.get_updates(payload.height).await.iter() {
                        state.cast(Event::SnarkCanonicitySummary(update_payload.clone()))?;
                    }

                    if let Err(e) = manager.prune().await {
                        eprintln!("{}", e);
                    }
                }
            }
            Event::MainnetBlock(event_payload) => {
                let manager = self.canonical_items_manager.lock().await;
                manager
                    .add_items_count(event_payload.height, &event_payload.state_hash, event_payload.snark_work_count as u64)
                    .await;
            }
            Event::SnarkWorkSummary(event_payload) => {
                {
                    let manager = self.canonical_items_manager.lock().await;
                    manager
                        .add_item(SnarkCanonicitySummaryPayload {
                            height: event_payload.height,
                            state_hash: event_payload.state_hash.to_string(),
                            timestamp: event_payload.timestamp,
                            prover: event_payload.prover.to_string(),
                            fee_nanomina: event_payload.fee_nanomina,
                            canonical: false, //set default value
                        })
                        .await;
                }
                {
                    let manager = self.canonical_items_manager.lock().await;
                    for update_payload in manager.get_updates(event_payload.height).await.iter() {
                        state.cast(Event::SnarkCanonicitySummary(update_payload.clone()))?;
                    }

                    if let Err(e) = manager.prune().await {
                        eprintln!("{}", e);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod snark_canonicity_summary_actor_tests {
    use super::*;
    use crate::event_sourcing::test_utils::setup_test_actors;
    use ractor::rpc::CallResult;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_snark_canonicity_summary_actor_with_mainnet_block() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(SnarkCanonicitySummaryActor::default()).await;

        // Send a MainnetBlock event
        let mainnet_block = MainnetBlockPayload {
            height: 10,
            state_hash: "sample_hash".to_string(),
            snark_work_count: 2,
            ..Default::default()
        };
        actor.call(|_| Event::MainnetBlock(mainnet_block.clone()), Some(Duration::from_secs(1))).await?;

        // Send SnarkWorkSummary events
        let snark_work_events = [
            SnarkWorkSummaryPayload {
                height: 10,
                state_hash: "sample_hash".to_string(),
                timestamp: 123456,
                prover: "test_prover_1".to_string(),
                fee_nanomina: 250_000_000,
            },
            SnarkWorkSummaryPayload {
                height: 10,
                state_hash: "sample_hash".to_string(),
                timestamp: 123457,
                prover: "test_prover_2".to_string(),
                fee_nanomina: 500_000_000,
            },
        ];

        for snark in snark_work_events.iter() {
            actor.call(|_| Event::SnarkWorkSummary(snark.clone()), Some(Duration::from_secs(1))).await?;
        }

        // Send BlockCanonicityUpdate event
        let update = BlockCanonicityUpdatePayload {
            height: 10,
            state_hash: "sample_hash".to_string(),
            canonical: true,
            was_canonical: false,
        };
        let response = actor
            .call(|_| Event::BlockCanonicityUpdate(update.clone()), Some(Duration::from_secs(1)))
            .await?;

        // Verify the response
        match response {
            CallResult::Success(Event::SnarkCanonicitySummary(payload)) => {
                assert_eq!(payload.height, 10);
                assert_eq!(payload.state_hash, "sample_hash");
                assert!(payload.canonical);
            }
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_does_not_publish_without_all_conditions_met() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(SnarkCanonicitySummaryActor::default()).await;

        // Send MainnetBlock with snark_work_count of 2
        let mainnet_block = MainnetBlockPayload {
            height: 10,
            state_hash: "sample_hash".to_string(),
            snark_work_count: 2,
            ..Default::default()
        };
        actor.call(|_| Event::MainnetBlock(mainnet_block.clone()), Some(Duration::from_secs(1))).await?;

        // Send only one SnarkWorkSummary (not enough)
        let snark_work = SnarkWorkSummaryPayload {
            height: 10,
            state_hash: "sample_hash".to_string(),
            timestamp: 123456,
            prover: "test_prover_1".to_string(),
            fee_nanomina: 250_000_000,
        };
        actor
            .call(|_| Event::SnarkWorkSummary(snark_work.clone()), Some(Duration::from_secs(1)))
            .await?;

        // Send BlockCanonicityUpdate
        let update = BlockCanonicityUpdatePayload {
            height: 10,
            state_hash: "sample_hash".to_string(),
            canonical: true,
            was_canonical: false,
        };
        let response = actor
            .call(|_| Event::BlockCanonicityUpdate(update.clone()), Some(Duration::from_secs(1)))
            .await?;

        // Verify no event is published
        match response {
            CallResult::Success(_) => panic!("Unexpected message type received"),
            CallResult::Timeout => {
                // Expected outcome: no event should be published
            }
            CallResult::SenderError => panic!("Call failed"),
        }

        Ok(())
    }
}
