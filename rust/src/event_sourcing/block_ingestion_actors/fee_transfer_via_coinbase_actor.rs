use crate::event_sourcing::{
    events::Event,
    payloads::{InternalCommandLogPayload, InternalCommandType},
};
use ractor::{Actor, ActorProcessingErr, ActorRef};

#[derive(Default)]
pub struct FeeTransferViaCoinbaseActor;

#[async_trait::async_trait]
impl Actor for FeeTransferViaCoinbaseActor {
    type Msg = Event;
    type State = ActorRef<Event>;
    type Arguments = ActorRef<Event>;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, parent: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        Ok(parent)
    }

    async fn handle(&self, _: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match msg {
            Event::MainnetBlock(block_payload) => {
                if let Some(fee_transfers_via_coinbase) = block_payload.fee_transfer_via_coinbase {
                    for fee_transfer_via_coinbase in fee_transfers_via_coinbase.iter() {
                        let payload = InternalCommandLogPayload {
                            internal_command_type: InternalCommandType::FeeTransferViaCoinbase,
                            height: block_payload.height,
                            state_hash: block_payload.state_hash.to_string(),
                            timestamp: block_payload.timestamp,
                            recipient: fee_transfer_via_coinbase.receiver.to_string(),
                            amount_nanomina: (fee_transfer_via_coinbase.fee * 1_000_000_000f64) as u64,
                            source: Some(block_payload.coinbase_receiver.to_string()),
                        };
                        state.cast(Event::InternalCommandLog(payload))?;
                    }
                }
            }
            Event::BerkeleyBlock(_) => {
                todo!("impl for berkeley block");
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_sourcing::{mainnet_block_models::*, payloads::MainnetBlockPayload, test_utils::setup_test_actors};
    use ractor::rpc::CallResult;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_fee_transfer_via_coinbase_actor_handle_event() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(FeeTransferViaCoinbaseActor {}).await;

        // MainnetBlockPayload with fee transfers via coinbase
        let fee_transfers_via_coinbase = vec![
            FeeTransferViaCoinbase {
                receiver: "receiver_1".to_string(),
                fee: 0.5,
            },
            FeeTransferViaCoinbase {
                receiver: "receiver_2".to_string(),
                fee: 0.3,
            },
        ];

        let block_payload = MainnetBlockPayload {
            height: 15,
            state_hash: "state_hash_example".to_string(),
            previous_state_hash: "previous_state_hash_example".to_string(),
            last_vrf_output: "last_vrf_output_example".to_string(),
            timestamp: 1615986540000,
            coinbase_receiver: "coinbase_receiver".to_string(),
            fee_transfer_via_coinbase: Some(fee_transfers_via_coinbase),
            ..Default::default()
        };

        // Send test message and store response
        let response = actor.call(|_| Event::MainnetBlock(block_payload.clone()), Some(Duration::from_secs(1))).await?;

        // Verify the fee transfers via coinbase
        for fee_transfer in block_payload.fee_transfer_via_coinbase.unwrap().iter() {
            match response {
                CallResult::Success(Event::InternalCommandLog(command_payload)) => {
                    assert_eq!(command_payload.internal_command_type, InternalCommandType::FeeTransferViaCoinbase);
                    assert_eq!(command_payload.height, block_payload.height);
                    assert_eq!(command_payload.state_hash, block_payload.state_hash);
                    assert_eq!(command_payload.timestamp, block_payload.timestamp);
                    assert_eq!(command_payload.recipient, fee_transfer.receiver);
                    assert_eq!(command_payload.amount_nanomina, (fee_transfer.fee * 1_000_000_000f64) as u64);
                    assert_eq!(command_payload.source, Some(block_payload.coinbase_receiver.to_string()));
                }
                CallResult::Success(_) => panic!("Received unexpected message type"),
                CallResult::Timeout => panic!("Call timed out"),
                CallResult::SenderError => panic!("Call failed"),
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_fee_transfer_via_coinbase_actor_no_fee_transfers() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(FeeTransferViaCoinbaseActor {}).await;

        // MainnetBlockPayload with no fee transfers via coinbase
        let block_payload = MainnetBlockPayload {
            height: 15,
            state_hash: "state_hash_example".to_string(),
            previous_state_hash: "previous_state_hash_example".to_string(),
            last_vrf_output: "last_vrf_output_example".to_string(),
            timestamp: 1615986540000,
            coinbase_receiver: "coinbase_receiver".to_string(),
            fee_transfer_via_coinbase: None,
            ..Default::default()
        };

        // Send test message and store response
        let response = actor.call(|_| Event::MainnetBlock(block_payload.clone()), Some(Duration::from_secs(1))).await?;

        // Verify no InternalCommandLog event is generated
        match response {
            CallResult::Success(Event::MainnetBlock(_)) => {}
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        Ok(())
    }
}
