use crate::stream::events::Event;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

pub struct TransitionFrontierActor {
    height: Arc<AtomicU64>,
}

impl TransitionFrontierActor {
    pub fn new() -> Self {
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
        if let Event::ActorHeight(actor_height) = msg {
            let current_height = self.height.load(Ordering::SeqCst);
            if actor_height.height > current_height {
                self.height.store(actor_height.height, Ordering::SeqCst);
                state.cast(Event::TransitionFrontier(actor_height.height))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::{payloads::ActorHeightPayload, test_utils::setup_test_actors};
    use ractor::rpc::CallResult;
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
