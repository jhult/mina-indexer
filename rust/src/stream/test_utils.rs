use crate::stream::events::Event;
use ractor::{Actor, ActorProcessingErr, ActorRef};

// Test supervisor that captures messages for verification
#[derive(Default)]
pub struct TestSupervisor {
    messages: Vec<Event>,
}

#[async_trait::async_trait]
impl Actor for TestSupervisor {
    type Msg = Event;
    type State = Self;
    type Arguments = ();

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, _parent: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        Ok(Self::default())
    }

    async fn handle(&self, _: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        state.messages.push(msg.clone());
        // Return the message through call() if it's being waited for
        Ok(())
    }
}

impl TestSupervisor {
    pub fn get_messages(&self) -> &Vec<Event> {
        &self.messages
    }

    pub fn get_last_message(&self) -> Option<&Event> {
        self.messages.last()
    }
}

// Helper function to setup test environment
pub async fn setup_test_actors<A>(actor: A) -> ActorRef<A::Msg>
where
    A: Actor,
    A::Arguments: From<ActorRef<Event>>,
{
    // Spawn supervisor
    let (supervisor, _) = Actor::spawn(None, TestSupervisor::default(), ())
        .await
        .expect("Failed to spawn test supervisor");

    // Spawn actor with supervisor reference
    let (actor_ref, _) = Actor::spawn(None, actor, supervisor.clone().into()).await.expect("Failed to spawn actor");

    actor_ref
}
