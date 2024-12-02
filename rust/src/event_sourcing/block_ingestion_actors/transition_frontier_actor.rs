use crate::{
    constants::TRANSITION_FRONTIER_DISTANCE,
    event_sourcing::{events::Event, models::Height, payloads::BlockCanonicityUpdatePayload},
};
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::{atomic::AtomicU64, Arc};

pub struct TransitionFrontierActor {
    height: Arc<AtomicU64>,
}

impl Default for TransitionFrontierActor {
    fn default() -> Self {
        Self {
            height: Arc::new(AtomicU64::new(0)),
        }
    }
}

#[async_trait::async_trait]
impl Actor for TransitionFrontierActor {
    type Msg = Event;
    type State = ActorRef<Event>;
    type Arguments = ActorRef<Event>;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, parent: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        Ok(parent)
    }

    async fn handle(&self, _myself: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        fn get_transition_frontier(payload: &BlockCanonicityUpdatePayload) -> u64 {
            payload.height - TRANSITION_FRONTIER_DISTANCE as u64
        }
        if let Event::BestBlock(payload) = msg {
            let mut transition_frontier = self.transition_frontier.lock().await;

            match &mut *transition_frontier {
                Some(tf) if payload.height > tf.0 + TRANSITION_FRONTIER_DISTANCE as u64 => {
                    tf.0 = get_transition_frontier(&payload);
                }
                None if payload.height > TRANSITION_FRONTIER_DISTANCE as u64 => *transition_frontier = Some(Height(get_transition_frontier(&payload))),
                _ => return Ok(()), // Early return if no action is needed
            }
            state.cast(Event::TransitionFrontier(get_transition_frontier(&payload)));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_sourcing::{payloads::ActorHeightPayload, test_utils::setup_test_actors};
    use ractor::rpc::CallResult;
    use std::cmp::Ordering;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_transition_frontier_actor_updates_height() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(TransitionFrontierActor::new()).await;

        let actor_height = ActorHeightPayload {
            actor: "TestActor".to_string(),
            height: 100,
        };

        let response = actor.call(|_| Event::ActorHeight(actor_height), Some(Duration::from_secs(1))).await?;

        match response {
            CallResult::Success(Event::TransitionFrontier(height)) => {
                assert_eq!(height, 100);
            }
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_transition_frontier_actor_ignores_lower_height() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(TransitionFrontierActor::new()).await;

        // Send higher height first
        let higher_height = ActorHeightPayload {
            actor: "TestActor".to_string(),
            height: 100,
        };

        let response = actor.call(|_| Event::ActorHeight(higher_height), Some(Duration::from_secs(1))).await?;

        match response {
            CallResult::Success(Event::TransitionFrontier(height)) => {
                assert_eq!(height, 100);
            }
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        // Send lower height
        let lower_height = ActorHeightPayload {
            actor: "TestActor".to_string(),
            height: 50,
        };

        let response = actor.call(|_| Event::ActorHeight(lower_height), Some(Duration::from_secs(1))).await?;

        match response {
            CallResult::Success(Event::ActorHeight(payload)) => {
                // Verify the height hasn't changed
                assert_eq!(payload.height.load(Ordering::SeqCst), 100);
            }
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_transition_frontier_actor_multiple_updates() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(TransitionFrontierActor::new()).await;

        for i in 1..=3 {
            let height = i * 100;
            let actor_height = ActorHeightPayload {
                actor: "TestActor".to_string(),
                height,
            };

            let response = actor.call(|_| Event::ActorHeight(actor_height), Some(Duration::from_secs(1))).await?;

            match response {
                CallResult::Success(Event::TransitionFrontier(h)) => {
                    assert_eq!(h, height);
                }
                CallResult::Success(_) => panic!("Received unexpected message type"),
                CallResult::Timeout => panic!("Call timed out"),
                CallResult::SenderError => panic!("Call failed"),
            }

            assert_eq!(actor.height.load(Ordering::SeqCst), height);
        }

        Ok(())
    }
}
