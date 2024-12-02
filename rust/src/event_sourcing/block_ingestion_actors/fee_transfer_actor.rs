use crate::event_sourcing::{
    events::Event,
    payloads::{InternalCommandLogPayload, InternalCommandType},
};
use ractor::{Actor, ActorProcessingErr, ActorRef};

#[derive(Default)]
pub struct FeeTransferActor;

#[async_trait::async_trait]
impl Actor for FeeTransferActor {
    type Msg = Event;
    type State = ActorRef<Event>;
    type Arguments = ActorRef<Event>;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, parent: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        Ok(parent)
    }

    async fn handle(&self, _: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match msg {
            Event::MainnetBlock(block_payload) => {
                if block_payload.excess_block_fees > 0 {
                    let payload = InternalCommandLogPayload {
                        internal_command_type: InternalCommandType::FeeTransfer,
                        height: block_payload.height,
                        state_hash: block_payload.state_hash.clone(),
                        timestamp: block_payload.timestamp,
                        recipient: block_payload.coinbase_receiver,
                        amount_nanomina: block_payload.excess_block_fees,
                        source: None,
                    };
                    state.cast(Event::InternalCommandLog(payload))?;
                }

                for fee_transfer in block_payload.fee_transfers.iter() {
                    let payload = InternalCommandLogPayload {
                        internal_command_type: InternalCommandType::FeeTransfer,
                        height: block_payload.height,
                        state_hash: block_payload.state_hash.clone(),
                        timestamp: block_payload.timestamp,
                        recipient: fee_transfer.recipient.to_string(),
                        amount_nanomina: fee_transfer.fee_nanomina,
                        source: None,
                    };
                    state.cast(Event::InternalCommandLog(payload))?;
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
    use crate::event_sourcing::{
        mainnet_block_models::*,
        payloads::{InternalCommandLogPayload, MainnetBlockPayload},
        test_utils::setup_test_actors,
    };
    use ractor::rpc::CallResult;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_fee_transfer_actor_handle_event() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(FeeTransferActor {}).await;

        // Mock a MainnetBlockPayload with fee transfers
        let fee_transfers = vec![
            FeeTransfer {
                recipient: "recipient_1".to_string(),
                fee_nanomina: 100_000,
            },
            FeeTransfer {
                recipient: "recipient_2".to_string(),
                fee_nanomina: 200_000,
            },
        ];

        // MainnetBlockPayload with sample fee transfers
        let block_payload = MainnetBlockPayload {
            height: 15,
            state_hash: "state_hash_example".to_string(),
            previous_state_hash: "previous_state_hash_example".to_string(),
            last_vrf_output: "last_vrf_output_example".to_string(),
            fee_transfers,
            timestamp: 1615986540000,
            ..Default::default()
        };

        // Send test message and store response
        let response = actor.call(|_| Event::MainnetBlock(block_payload.clone()), Some(Duration::from_secs(1))).await?;

        // Capture and verify the published InternalCommand events for fee transfers
        for fee_transfer in block_payload.fee_transfers.iter() {
            match response {
                CallResult::Success(Event::InternalCommandLog(command_payload)) => {
                    assert_eq!(command_payload.internal_command_type, InternalCommandType::FeeTransfer);
                    assert_eq!(command_payload.height, block_payload.height);
                    assert_eq!(command_payload.state_hash, block_payload.state_hash);
                    assert_eq!(command_payload.timestamp, block_payload.timestamp);
                    assert_eq!(command_payload.recipient, fee_transfer.recipient);
                    assert_eq!(command_payload.amount_nanomina, fee_transfer.fee_nanomina);
                }
                CallResult::Success(_) => panic!("Received unexpected message type"),
                CallResult::Timeout => panic!("Call timed out"),
                CallResult::SenderError => panic!("Call failed"),
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_fee_transfer_actor_handle_event_with_coinbase_payment() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(FeeTransferActor {}).await;

        // Mock a MainnetBlockPayload with fee transfers and excess block fees
        let fee_transfers = vec![
            FeeTransfer {
                recipient: "recipient_1".to_string(),
                fee_nanomina: 100_000,
            },
            FeeTransfer {
                recipient: "recipient_2".to_string(),
                fee_nanomina: 200_000,
            },
        ];

        // Mock user commands with higher total fees paid
        let user_commands = vec![CommandSummary {
            sender: "sender_1".to_string(),
            fee_payer: "fee_payer_1".to_string(),
            fee_nanomina: 500_000,
            amount_nanomina: 1_000_000,
            receiver: "receiver_1".to_string(),
            nonce: 1,
            status: CommandStatus::Applied,
            txn_type: CommandType::Payment,
            memo: "".to_string(),
        }];

        // MainnetBlockPayload with fee transfers and user commands
        let block_payload = MainnetBlockPayload {
            height: 15,
            state_hash: "state_hash_example".to_string(),
            previous_state_hash: "previous_state_hash_example".to_string(),
            last_vrf_output: "last_vrf_output_example".to_string(),
            fee_transfers,
            user_commands,
            timestamp: 1615986540000,
            coinbase_receiver: "coinbase_receiver".to_string(),
            excess_block_fees: 200_000,
            ..Default::default()
        };

        // Send test message and store response
        let response = actor.call(|_| Event::MainnetBlock(block_payload.clone()), Some(Duration::from_secs(1))).await?;

        // Verify the payment to the coinbase receiver
        match response {
            CallResult::Success(Event::InternalCommandLog(command_payload)) => {
                assert_eq!(command_payload.internal_command_type, InternalCommandType::FeeTransfer);
                assert_eq!(command_payload.height, block_payload.height);
                assert_eq!(command_payload.state_hash, block_payload.state_hash);
                assert_eq!(command_payload.timestamp, block_payload.timestamp);
                assert_eq!(command_payload.recipient, block_payload.coinbase_receiver);
                assert_eq!(command_payload.amount_nanomina, block_payload.excess_block_fees);
            }
            CallResult::Success(_) => panic!("Received unexpected message type"),
            CallResult::Timeout => panic!("Call timed out"),
            CallResult::SenderError => panic!("Call failed"),
        }

        // Verify the fee transfer events
        for fee_transfer in block_payload.fee_transfers.iter() {
            let response = actor.call(|_| Event::MainnetBlock(block_payload.clone()), Some(Duration::from_secs(1))).await?;
            match response {
                CallResult::Success(Event::InternalCommandLog(command_payload)) => {
                    assert_eq!(command_payload.internal_command_type, InternalCommandType::FeeTransfer);
                    assert_eq!(command_payload.height, block_payload.height);
                    assert_eq!(command_payload.state_hash, block_payload.state_hash);
                    assert_eq!(command_payload.timestamp, block_payload.timestamp);
                    assert_eq!(command_payload.recipient, fee_transfer.recipient);
                    assert_eq!(command_payload.amount_nanomina, fee_transfer.fee_nanomina);
                }
                CallResult::Success(_) => panic!("Received unexpected message type"),
                CallResult::Timeout => panic!("Call timed out"),
                CallResult::SenderError => panic!("Call failed"),
            }
        }

        Ok(())
    }
}
