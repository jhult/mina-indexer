use super::super::events::Event;
use crate::event_sourcing::payloads::UserCommandLogPayload;
use ractor::{Actor, ActorProcessingErr, ActorRef};

#[derive(Default)]
pub struct UserCommandLogActor;

#[async_trait::async_trait]
impl Actor for UserCommandLogActor {
    type Msg = Event;
    type State = ActorRef<Event>;
    type Arguments = ActorRef<Event>;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, parent: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        Ok(parent)
    }

    async fn handle(&self, _myself: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match msg {
            Event::MainnetBlock(block_payload) => {
                for user_command in block_payload.user_commands.iter() {
                    let payload = UserCommandLogPayload {
                        height: block_payload.height,
                        global_slot: block_payload.global_slot,
                        txn_hash: user_command.txn_hash(),
                        state_hash: block_payload.state_hash.to_string(),
                        timestamp: block_payload.timestamp,
                        txn_type: user_command.txn_type.clone(),
                        status: user_command.status.clone(),
                        sender: user_command.sender.to_string(),
                        receiver: user_command.receiver.to_string(),
                        nonce: user_command.nonce,
                        fee_nanomina: user_command.fee_nanomina,
                        fee_payer: user_command.fee_payer.to_string(),
                        amount_nanomina: user_command.amount_nanomina,
                    };
                    state.cast(Event::UserCommandLog(payload))?;
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
        payloads::{MainnetBlockPayload, UserCommandLogPayload},
        test_utils::setup_test_actors,
    };
    use ractor::rpc::CallResult;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_user_command_actor_handle_event() -> anyhow::Result<()> {
        // Setup test actors
        let actor = setup_test_actors(UserCommandLogActor {}).await;

        // Mock a MainnetBlockPayload with user commands
        let user_commands = vec![
            CommandSummary {
                memo: "memo_1".to_string(),
                fee_payer: "payer_1".to_string(),
                sender: "sender_1".to_string(),
                receiver: "receiver_1".to_string(),
                status: CommandStatus::Applied,
                txn_type: CommandType::Payment,
                nonce: 1,
                fee_nanomina: 10_000_000,
                amount_nanomina: 500_000_000,
            },
            CommandSummary {
                memo: "memo_2".to_string(),
                fee_payer: "payer_2".to_string(),
                sender: "sender_2".to_string(),
                receiver: "receiver_2".to_string(),
                status: CommandStatus::Failed,
                txn_type: CommandType::StakeDelegation,
                nonce: 2,
                fee_nanomina: 5_000_000,
                amount_nanomina: 0,
            },
        ];

        // MainnetBlockPayload with sample user commands
        let block_payload = MainnetBlockPayload {
            height: 10,
            state_hash: "state_hash".to_string(),
            previous_state_hash: "previous_state_hash".to_string(),
            last_vrf_output: "last_vrf_output".to_string(),
            user_command_count: 2,
            user_commands,
            snark_work_count: 0,
            snark_work: vec![],
            timestamp: 123414312431234,
            coinbase_receiver: "coinbase_receiver".to_string(),
            coinbase_reward_nanomina: 720_000_000_000,
            global_slot_since_genesis: 16,
            ..Default::default()
        };

        // Send test message and store response
        let response = actor.call(|_| Event::MainnetBlock(block_payload.clone()), Some(Duration::from_secs(1))).await?;

        // Verify the response for each user command
        for user_command in block_payload.user_commands.iter() {
            match response {
                CallResult::Success(Event::UserCommandLog(summary_payload)) => {
                    assert_eq!(summary_payload.height, block_payload.height);
                    assert_eq!(summary_payload.state_hash, block_payload.state_hash);
                    assert_eq!(summary_payload.timestamp, block_payload.timestamp);
                    assert_eq!(summary_payload.txn_type, user_command.txn_type);
                    assert_eq!(summary_payload.status, user_command.status);
                    assert_eq!(summary_payload.sender, user_command.sender);
                    assert_eq!(summary_payload.receiver, user_command.receiver);
                    assert_eq!(summary_payload.nonce, user_command.nonce);
                    assert_eq!(summary_payload.fee_nanomina, user_command.fee_nanomina);
                    assert_eq!(summary_payload.amount_nanomina, user_command.amount_nanomina);
                }
                CallResult::Success(_) => panic!("Received unexpected message type"),
                CallResult::Timeout => panic!("Call timed out"),
                CallResult::SenderError => panic!("Call failed"),
            }
        }

        Ok(())
    }
}
