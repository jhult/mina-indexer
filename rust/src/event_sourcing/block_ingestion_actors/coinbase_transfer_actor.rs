use super::super::events::Event;
use crate::event_sourcing::payloads::{InternalCommandLogPayload, InternalCommandType};
use ractor::{Actor, ActorProcessingErr, ActorRef};

#[derive(Default)]
pub struct CoinbaseTransferActor;

#[async_trait::async_trait]
impl Actor for CoinbaseTransferActor {
    type Msg = Event;
    type State = ActorRef<Event>;
    type Arguments = ActorRef<Event>;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, parent: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        Ok(parent)
    }

    async fn handle(&self, _myself: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match msg {
            Event::MainnetBlock(block_payload) => {
                let payload = InternalCommandLogPayload {
                    internal_command_type: InternalCommandType::Coinbase,
                    height: block_payload.height,
                    state_hash: block_payload.state_hash.to_string(),
                    timestamp: block_payload.timestamp,
                    recipient: block_payload.coinbase_receiver,
                    amount_nanomina: block_payload.coinbase_reward_nanomina,
                    source: None,
                };
                state.cast(Event::InternalCommandLog(payload));
            }
            Event::BerkeleyBlock(block) => {
                todo!("impl for berkeley block");
            }
            _ => {}
        }
        Ok(())
    }
}
