use crate::{constants::HEIGHT_SPREAD_MSG_THROTTLE, stream::actors::blockchain_tree_builder_actor::BlockchainTreeBuilderActor};
use actors::{
    berkeley_block_parser_actor::BerkeleyBlockParserActor, best_block_actor::BestBlockActor, block_ancestor_actor::BlockAncestorActor,
    block_canonicity_actor::BlockCanonicityActor, block_confirmations_actor::BlockConfirmationsActor, block_log_actor::BlockLogActor,
    canonical_block_log_actor::CanonicalBlockLogActor, canonical_block_log_persistence_actor::CanonicalBlockLogPersistenceActor,
    canonical_internal_command_log_actor::CanonicalInternalCommandLogActor,
    canonical_internal_command_log_persistence_actor::CanonicalInternalCommandLogPersistenceActor,
    canonical_user_command_log_actor::CanonicalUserCommandLogActor, canonical_user_command_persistence_actor::CanonicalUserCommandPersistenceActor,
    coinbase_transfer_actor::CoinbaseTransferActor, fee_transfer_actor::FeeTransferActor, fee_transfer_via_coinbase_actor::FeeTransferViaCoinbaseActor,
    ledger_actor::LedgerActor, mainnet_block_parser_actor::MainnetBlockParserActor, monitor_actor::MonitorActor, new_account_actor::NewAccountActor,
    pcb_path_actor::PCBBlockPathActor, snark_canonicity_summary_actor::SnarkCanonicitySummaryActor,
    snark_summary_persistence_actor::SnarkSummaryPersistenceActor, snark_work_actor::SnarkWorkSummaryActor, transition_frontier_actor::TransitionFrontierActor,
    user_command_log_actor::UserCommandLogActor,
};
use ractor::{Actor, ActorProcessingErr, ActorRef};

mod actors;
pub mod berkeley_block_models;
pub mod canonical_items_manager;
pub mod db_logger;
pub mod events;
pub mod genesis_ledger_models;
pub mod mainnet_block_models;
pub mod models;
pub mod partitioned_table;
pub mod payloads;
pub mod sourcing;

#[cfg(test)]
pub mod test_utils;

pub struct Supervisor {
    accounting_actor: ActorRef<AccountingPayload>,
    berkeley_block_parser_actor: ActorRef<BerkeleyBlockParserActor>,
    best_block_actor: ActorRef<BestBlockActor>,
    block_ancestor_actor: ActorRef<BlockAncestorActor>,
    block_canonicity_actor: ActorRef<BlockCanonicityActor>,
    block_confirmations_actor: ActorRef<BlockConfirmationsActor>,
    block_log_actor: ActorRef<BlockLogActor>,
    blockchain_tree_builder_actor: ActorRef<BlockchainTreeBuilderActor>,
    canonical_block_log_actor: ActorRef<CanonicalBlockLogActor>,
    canonical_block_log_persistence_actor: ActorRef<CanonicalBlockLogPersistenceActor>,
    canonical_internal_command_log_actor: ActorRef<CanonicalInternalCommandLogActor>,
    canonical_internal_command_log_persistence_actor: ActorRef<CanonicalInternalCommandLogPersistenceActor>,
    canonical_user_command_log_actor: ActorRef<CanonicalUserCommandLogActor>,
    canonical_user_command_persistence_actor: ActorRef<CanonicalUserCommandPersistenceActor>,
    coinbase_transfer_actor: ActorRef<CoinbaseTransferActor>,
    fee_transfer_actor: ActorRef<FeeTransferActor>,
    fee_transfer_via_coinbase_actor: ActorRef<FeeTransferViaCoinbaseActor>,
    ledger_actor: ActorRef<LedgerActor>,
    mainnet_block_parser_actor: ActorRef<MainnetBlockParserActor>,
    new_account_actor: ActorRef<NewAccountActor>,
    pcb_path_actor: ActorRef<PCBBlockPathActor>,
    snark_canonicity_summary_actor: ActorRef<SnarkCanonicitySummaryActor>,
    snark_summary_persistence_actor: ActorRef<SnarkSummaryPersistenceActor>,
    snark_work_actor: ActorRef<SnarkWorkSummaryActor>,
    transition_frontier_actor: ActorRef<TransitionFrontierActor>,
    user_command_log_actor: ActorRef<UserCommandLogActor>,
}

#[async_trait::async_trait]
impl Actor for Supervisor {
    type Msg = ();
    type State = Supervisor;
    type Arguments = ();

    async fn pre_start(&self, myself: ActorRef<Self::Msg>, args: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        // Spawn all child actors
        let (accounting_actor, _) = Actor::spawn("AccountingActor", Some(myself.clone()), ()).await?;
        let (berkeley_block_parser_actor, _) = Actor::spawn("BerkeleyBlockParserActor", Some(myself.clone()), ()).await?;
        let (best_block_actor, _) = Actor::spawn("BestBlockActor", Some(myself.clone()), ()).await?;
        let (block_ancestor_actor, _) = Actor::spawn("BlockAncestorActor", Some(myself.clone()), ()).await?;
        let (block_canonicity_actor, _) = Actor::spawn("BlockCanonicityActor", Some(myself.clone()), ()).await?;
        let (block_confirmations_actor, _) = Actor::spawn("BlockConfirmationsActor", Some(myself.clone()), ()).await?;
        let (block_log_actor, _) = Actor::spawn("BlockLogActor", Some(myself.clone()), ()).await?;
        let (blockchain_tree_builder_actor, _) = Actor::spawn("BlockchainTreeBuilderActor", Some(myself.clone()), ()).await?;
        let (canonical_block_log_actor, _) = Actor::spawn("CanonicalBlockLogActor", Some(myself.clone()), ()).await?;
        let (canonical_block_log_persistence_actor, _) = Actor::spawn("CanonicalBlockLogPersistenceActor", Some(myself.clone()), ()).await?;
        let (canonical_internal_command_log_actor, _) = Actor::spawn("CanonicalInternalCommandLogActor", Some(myself.clone()), ()).await?;
        let (canonical_internal_command_log_persistence_actor, _) =
            Actor::spawn("CanonicalInternalCommandLogPersistenceActor", Some(myself.clone()), ()).await?;
        let (canonical_user_command_log_actor, _) = Actor::spawn("CanonicalUserCommandLogActor", Some(myself.clone()), ()).await?;
        let (canonical_user_command_persistence_actor, _) = Actor::spawn("CanonicalUserCommandPersistenceActor", Some(myself.clone()), ()).await?;
        let (coinbase_transfer_actor, _) = Actor::spawn("CoinbaseTransferActor", Some(myself.clone()), ()).await?;
        let (fee_transfer_actor, _) = Actor::spawn("FeeTransferActor", Some(myself.clone()), ()).await?;
        let (fee_transfer_via_coinbase_actor, _) = Actor::spawn("FeeTransferViaCoinbaseActor", Some(myself.clone()), ()).await?;
        let (ledger_actor, _) = Actor::spawn("LedgerActor", Some(myself.clone()), ()).await?;
        let (mainnet_block_parser_actor, _) = Actor::spawn("MainnetBlockParserActor", Some(myself.clone()), ()).await?;
        let (new_account_actor, _) = Actor::spawn("NewAccountActor", Some(myself.clone()), ()).await?;
        let (pcb_path_actor, _) = Actor::spawn("PCBBlockPathActor", Some(myself.clone()), ()).await?;
        let (snark_canonicity_summary_actor, _) = Actor::spawn("SnarkCanonicitySummaryActor", Some(myself.clone()), ()).await?;
        let (snark_summary_persistence_actor, _) = Actor::spawn("SnarkSummaryPersistenceActor", Some(myself.clone()), ()).await?;
        let (snark_work_actor, _) = Actor::spawn("SnarkWorkSummaryActor", Some(myself.clone()), ()).await?;
        let (transition_frontier_actor, _) = Actor::spawn("TransitionFrontierActor", Some(myself.clone()), ()).await?;
        let (user_command_log_actor, _) = Actor::spawn("UserCommandLogActor", Some(myself.clone()), ()).await?;

        Ok(Supervisor {
            accounting_actor,
            berkeley_block_parser_actor,
            best_block_actor,
            block_ancestor_actor,
            block_canonicity_actor,
            block_confirmations_actor,
            block_log_actor,
            blockchain_tree_builder_actor,
            canonical_block_log_actor,
            canonical_block_log_persistence_actor,
            canonical_internal_command_log_actor,
            canonical_internal_command_log_persistence_actor,
            canonical_user_command_log_actor,
            canonical_user_command_persistence_actor,
            coinbase_transfer_actor,
            fee_transfer_actor,
            fee_transfer_via_coinbase_actor,
            ledger_actor,
            mainnet_block_parser_actor,
            new_account_actor,
            pcb_path_actor,
            snark_canonicity_summary_actor,
            snark_summary_persistence_actor,
            snark_work_actor,
            transition_frontier_actor,
            user_command_log_actor,
        })
    }

    async fn handle(&self, _myself: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match msg {
            SystemMessage::Block(payload) => {
                state.block_actor.send_message(payload)?;
            }
            SystemMessage::UserCommand(payload) => {
                state.user_cmd_actor.send_message(payload)?;
            }
            SystemMessage::Accounting(payload) => {
                state.accounting_actor.send_message(payload)?;
            }
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_process_blocks_dir_with_mainnet_blocks() -> anyhow::Result<()> {
    use crate::stream::{events::Event, payloads::*, sourcing::*};
    use std::{collections::HashMap, path::PathBuf, str::FromStr};
    use tokio::{sync::broadcast, time::Duration};

    // Create a shutdown channel for the test
    let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

    let shared_publisher = Arc::new(SharedPublisher::new(100_000)); // Initialize publisher
    let mut receiver = shared_publisher.subscribe();

    // Spawn the task to process blocks
    let process_handle = tokio::spawn({
        let shared_publisher = Arc::clone(&shared_publisher);
        let shutdown_receiver = shutdown_receiver.resubscribe();
        async move {
            subscribe_actors(&shared_publisher, shutdown_receiver, None).await.unwrap();
        }
    });

    let blocks_dir = PathBuf::from_str("./src/stream/test_data/100_mainnet_blocks").expect("Directory with mainnet blocks should exist");
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    publish_genesis_block(&shared_publisher).unwrap();
    publish_block_dir_paths(blocks_dir, &shared_publisher, shutdown_receiver, None).await?;

    // Wait a short duration for some events to be processed, then trigger shutdown
    tokio::time::sleep(Duration::from_secs(1)).await;
    let _ = shutdown_sender.send(());

    // Wait for the task to finish processing
    let _ = process_handle.await;

    // Count each event type received
    let mut event_counts: HashMap<Event, usize> = HashMap::new();
    let mut internal_command_counts: HashMap<InternalCommandType, usize> = HashMap::new();
    let mut last_best_block: Option<BlockCanonicityUpdatePayload> = None;
    while let Ok(event) = receiver.try_recv() {
        if event.event_type == Event::BestBlock {
            last_best_block = Some(sonic_rs::from_str(&event.payload).unwrap());
        }
        match event.event_type {
            Event::InternalCommandLog => {
                if let Ok(InternalCommandLogPayload { internal_command_type, .. }) = sonic_rs::from_str(&event.payload) {
                    *internal_command_counts.entry(internal_command_type).or_insert(0) += 1
                }
            }
            _ => {
                *event_counts.entry(event.event_type).or_insert(0) += 1;
            }
        }
    }

    let paths_count = 165;
    let paths_plus_genesis_count = paths_count + 1;
    let length_of_chain = 100;
    let number_of_user_commands = 247; // hand-calulated

    assert_eq!(event_counts.get(&Event::PrecomputedBlockPath).cloned().unwrap(), paths_count);
    assert_eq!(event_counts.get(&Event::MainnetBlockPath).cloned().unwrap(), paths_count);
    assert_eq!(event_counts.get(&Event::BlockAncestor).cloned().unwrap(), paths_count);
    assert_eq!(event_counts.get(&Event::NewBlock).cloned().unwrap(), paths_plus_genesis_count);
    assert_eq!(event_counts.get(&Event::BlockLog).cloned().unwrap(), paths_plus_genesis_count);
    assert_eq!(event_counts.get(&Event::UserCommandLog).cloned().unwrap(), number_of_user_commands);

    assert!(event_counts.get(&Event::BestBlock).cloned().unwrap() > length_of_chain);
    assert!(event_counts.get(&Event::BestBlock).cloned().unwrap() < paths_count);
    assert!(!event_counts.contains_key(&Event::TransitionFrontier));

    assert_eq!(internal_command_counts.get(&InternalCommandType::Coinbase).cloned().unwrap(), paths_count);
    assert_eq!(internal_command_counts.get(&InternalCommandType::FeeTransfer).cloned().unwrap(), 159); //manual count reveals 161
    assert!(!internal_command_counts.contains_key(&InternalCommandType::FeeTransferViaCoinbase));

    // Best Block & Last canonical update:
    assert_eq!(last_best_block.clone().unwrap().height, length_of_chain as u64);
    assert_eq!(&last_best_block.unwrap().state_hash, "3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4");

    Ok(())
}

#[tokio::test]
async fn test_process_blocks_dir_canonical_updates() -> anyhow::Result<()> {
    use crate::stream::{events::Event, payloads::BlockCanonicityUpdatePayload, sourcing::*};
    use std::{path::PathBuf, str::FromStr};
    use tokio::{sync::broadcast, time::Duration};

    // Create a shutdown channel for the test
    let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

    let shared_publisher = Arc::new(SharedPublisher::new(100_000)); // Initialize publisher
    let mut receiver = shared_publisher.subscribe();

    // Spawn the task to process blocks
    let process_handle = tokio::spawn({
        let shared_publisher = Arc::clone(&shared_publisher);
        let shutdown_receiver = shutdown_receiver.resubscribe();
        async move {
            subscribe_actors(&shared_publisher, shutdown_receiver, None).await.unwrap();
        }
    });

    let blocks_dir = PathBuf::from_str("./src/stream/test_data/10_mainnet_blocks").expect("Directory with mainnet blocks should exist");
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    publish_genesis_block(&shared_publisher).unwrap();
    publish_block_dir_paths(blocks_dir, &shared_publisher, shutdown_receiver, None).await?;

    // Wait a short duration for some events to be processed, then trigger shutdown
    tokio::time::sleep(Duration::from_secs(5)).await;
    let _ = shutdown_sender.send(());

    // Wait for the task to finish processing
    let _ = process_handle.await;

    // Define the expected canonicity events based on the detailed log entries provided
    let expected_canonical_events = vec![
        // first at height: canonical
        (1u64, String::from("3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ"), true),
        // first at height: canonical
        (2u64, String::from("3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH"), true),
        // first at height: canonical
        (3u64, String::from("3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R"), true),
        // first at height: canonical
        (4u64, String::from("3NL9qBsNibXPm5Nh8cSg5CCqrbzX5VUVY9gJzAbg7EVCF3hfhazG"), true),
        // first at height: canonical
        (5u64, String::from("3NKQUoBfi9vkbuqtDJmSEYBQrcSo4GjwG8bPCiii4yqM8AxEQvtY"), true),
        // first at height: canonical
        (6u64, String::from("3NKqMEewA8gvEiW7So7nZ3DN6tPnmCtHpWuAzADN5ff9wiqkGf45"), true),
        // 3NKqRR2BZFV7Ad5kxtGKNNL59neXohf4ZEC5EMKrrnijB1jy4R5v has greater last_vrf_output
        (6u64, String::from("3NKqMEewA8gvEiW7So7nZ3DN6tPnmCtHpWuAzADN5ff9wiqkGf45"), false),
        (6u64, String::from("3NKqRR2BZFV7Ad5kxtGKNNL59neXohf4ZEC5EMKrrnijB1jy4R5v"), true),
        // last_vrf_ouput lexicographically smaller than 3NKqRR2BZFV7Ad5kxtGKNNL59neXohf4ZEC5EMKrrnijB1jy4R5v
        (6u64, String::from("3NKvdydTvLVDJ9PKAXrisjsXoZQvUy1V2sbComWyB2uyhARCJZ5M"), false),
        // last_vrf_ouput lexicographically smaller than 3NKqRR2BZFV7Ad5kxtGKNNL59neXohf4ZEC5EMKrrnijB1jy4R5v
        (6u64, String::from("3NKqRR2BZFV7Ad5kxtGKNNL59neXohf4ZEC5EMKrrnijB1jy4R5v"), false),
        // 3NLM3k3Vk1qs36hZWdbWvi4sqwer3skbgPyHMWrZMBoscNLyjnY2 has greater last_vrf_output
        (6u64, String::from("3NLM3k3Vk1qs36hZWdbWvi4sqwer3skbgPyHMWrZMBoscNLyjnY2"), true),
        // block at height 7 links to 3NKqRR2BZFV7Ad5kxtGKNNL59neXohf4ZEC5EMKrrnijB1jy4R5v at height 6
        // resulting in branch competition. 3NKqRR2BZFV7Ad5kxtGKNNL59neXohf4ZEC5EMKrrnijB1jy4R5v at height 6
        // now becomes canonical, despite having smaller last_vrf_output compared to
        // 3NLM3k3Vk1qs36hZWdbWvi4sqwer3skbgPyHMWrZMBoscNLyjnY2
        (6u64, String::from("3NLM3k3Vk1qs36hZWdbWvi4sqwer3skbgPyHMWrZMBoscNLyjnY2"), false),
        (6u64, String::from("3NKqRR2BZFV7Ad5kxtGKNNL59neXohf4ZEC5EMKrrnijB1jy4R5v"), true),
        (7u64, String::from("3NL7dd6X6316xu6JtJj6cHwAhHrXwZC4SdBU9TUDUUhfAkB8cSoK"), true),
        (7u64, String::from("3NL7dd6X6316xu6JtJj6cHwAhHrXwZC4SdBU9TUDUUhfAkB8cSoK"), false),
        (7u64, String::from("3NLGcwFVQF1p1PrZpusw2fZwBe5HKXGtrGy1Vc4aPkeBtT8nMNUc"), true),
        (8u64, String::from("3NLVZQz4FwFbvW4hejfyRpw5NyP8XvQjhj4wSsCjCKdHNBjwWsPG"), true),
        (9u64, String::from("3NKK3QwQbAgMSmrHq4wpgqEwXp5pd9B18CMQjgYsjKTdq8CAsuM6"), true),
        (9u64, String::from("3NKYjQ6h8xw8RdYvGk8Rc3NnNQHLXjRczUDDZLCXkTJsZFHDhsH6"), false),
        (9u64, String::from("3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw"), false),
        (9u64, String::from("3NKK3QwQbAgMSmrHq4wpgqEwXp5pd9B18CMQjgYsjKTdq8CAsuM6"), false),
        (9u64, String::from("3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw"), true),
        (10u64, String::from("3NKGgTk7en3347KH81yDra876GPAUSoSePrfVKPmwR1KHfMpvJC5"), true),
        (10u64, String::from("3NKGgTk7en3347KH81yDra876GPAUSoSePrfVKPmwR1KHfMpvJC5"), false),
        (10u64, String::from("3NKHYHrqKpDcon6ToV5CLDiheanjshk5gcsNqefnK78phCFTR2aL"), true),
    ];

    // Collect actual BlockCanonicityUpdatePayload events received
    let mut actual_canonical_events = vec![];
    while let Ok(event) = receiver.try_recv() {
        if event.event_type == Event::BlockCanonicityUpdate {
            let payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(&event.payload).unwrap();
            actual_canonical_events.push((payload.height, payload.state_hash, payload.canonical));
        }
    }

    // Compare the actual and expected events
    assert_eq!(
        actual_canonical_events.len(),
        expected_canonical_events.len(),
        "Mismatch in the number of events"
    );
    assert_eq!(actual_canonical_events, expected_canonical_events, "Events do not match the expected sequence");

    Ok(())
}
