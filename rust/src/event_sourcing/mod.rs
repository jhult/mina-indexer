use crate::constants::HEIGHT_SPREAD_MSG_THROTTLE;
use block_ingestion_actors::{
    accounting_actor::AccountingActor, berkeley_block_parser_actor::BerkeleyBlockParserActor, best_block_actor::BestBlockActor,
    block_ancestor_actor::BlockAncestorActor, block_canonicity_actor::BlockCanonicityActor, block_confirmations_actor::BlockConfirmationsActor,
    block_log_actor::BlockLogActor, blockchain_tree_builder_actor::BlockchainTreeBuilderActor, canonical_block_log_actor::CanonicalBlockLogActor,
    canonical_block_log_persistence_actor::CanonicalBlockLogPersistenceActor, canonical_internal_command_log_actor::CanonicalInternalCommandLogActor,
    canonical_internal_command_log_persistence_actor::CanonicalInternalCommandLogPersistenceActor,
    canonical_user_command_log_actor::CanonicalUserCommandLogActor, canonical_user_command_persistence_actor::CanonicalUserCommandPersistenceActor,
    coinbase_transfer_actor::CoinbaseTransferActor, fee_transfer_actor::FeeTransferActor, fee_transfer_via_coinbase_actor::FeeTransferViaCoinbaseActor,
    ledger_actor::LedgerActor, mainnet_block_parser_actor::MainnetBlockParserActor, new_account_actor::NewAccountActor, pcb_path_actor::PCBBlockPathActor,
    snark_canonicity_summary_actor::SnarkCanonicitySummaryActor, snark_summary_persistence_actor::SnarkSummaryPersistenceActor,
    snark_work_actor::SnarkWorkSummaryActor, transition_frontier_actor::TransitionFrontierActor, user_command_log_actor::UserCommandLogActor,
};
use events::Event;
use ractor::{Actor, ActorProcessingErr, ActorRef, State};

pub mod berkeley_block_models;
mod block_ingestion_actors;
pub mod canonical_items_manager;
pub mod db_logger;
pub mod events;
pub mod genesis_ledger_models;
pub mod mainnet_block_models;
pub mod managed_table;
pub mod models;
pub mod payloads;
pub mod sourcing;
pub mod staking_ledger_actors;
pub mod staking_ledger_models;

#[cfg(test)]
pub mod test_utils;

pub struct Supervisor {
    accounting_actor: ActorRef<Event>,
    berkeley_block_parser_actor: ActorRef<Event>,
    best_block_actor: ActorRef<Event>,
    block_ancestor_actor: ActorRef<Event>,
    block_canonicity_actor: ActorRef<Event>,
    block_confirmations_actor: ActorRef<Event>,
    block_log_actor: ActorRef<Event>,
    blockchain_tree_builder_actor: ActorRef<Event>,
    canonical_block_log_actor: ActorRef<Event>,
    canonical_block_log_persistence_actor: ActorRef<Event>,
    canonical_internal_command_log_actor: ActorRef<Event>,
    canonical_internal_command_log_persistence_actor: ActorRef<Event>,
    canonical_user_command_log_actor: ActorRef<Event>,
    canonical_user_command_persistence_actor: ActorRef<Event>,
    coinbase_transfer_actor: ActorRef<Event>,
    fee_transfer_actor: ActorRef<Event>,
    fee_transfer_via_coinbase_actor: ActorRef<Event>,
    ledger_actor: ActorRef<Event>,
    mainnet_block_parser_actor: ActorRef<Event>,
    new_account_actor: ActorRef<Event>,
    pcb_path_actor: ActorRef<Event>,
    snark_canonicity_summary_actor: ActorRef<Event>,
    snark_summary_persistence_actor: ActorRef<Event>,
    snark_work_actor: ActorRef<Event>,
    transition_frontier_actor: ActorRef<Event>,
    user_command_log_actor: ActorRef<Event>,
}

#[async_trait::async_trait]
impl Actor for Supervisor {
    type Msg = Event;
    type State = Supervisor;
    type Arguments = ();

    async fn pre_start(&self, myself: ActorRef<Self::Msg>, _args: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        // Spawn all child actors
        let accounting_actor = spawn_actor::<AccountingActor>(myself.clone()).await?;
        let berkeley_block_parser_actor = spawn_actor::<BerkeleyBlockParserActor>(myself.clone()).await?;
        let best_block_actor = spawn_actor::<BestBlockActor>(myself.clone()).await?;
        let block_ancestor_actor = spawn_actor::<BlockAncestorActor>(myself.clone()).await?;
        let block_canonicity_actor = spawn_actor::<BlockCanonicityActor>(myself.clone()).await?;
        let block_confirmations_actor = spawn_actor::<BlockConfirmationsActor>(myself.clone()).await?;
        let block_log_actor = spawn_actor::<BlockLogActor>(myself.clone()).await?;
        let blockchain_tree_builder_actor = spawn_actor::<BlockchainTreeBuilderActor>(myself.clone()).await?;
        let canonical_block_log_actor = spawn_actor::<CanonicalBlockLogActor>(myself.clone()).await?;
        let canonical_block_log_persistence_actor = spawn_actor::<CanonicalBlockLogPersistenceActor>(myself.clone()).await?;
        let canonical_internal_command_log_actor = spawn_actor::<CanonicalInternalCommandLogActor>(myself.clone()).await?;
        let canonical_internal_command_log_persistence_actor = spawn_actor::<CanonicalInternalCommandLogPersistenceActor>(myself.clone()).await?;
        let canonical_user_command_log_actor = spawn_actor::<CanonicalUserCommandLogActor>(myself.clone()).await?;
        let canonical_user_command_persistence_actor = spawn_actor::<CanonicalUserCommandPersistenceActor>(myself.clone()).await?;
        let coinbase_transfer_actor = spawn_actor::<CoinbaseTransferActor>(myself.clone()).await?;
        let fee_transfer_actor = spawn_actor::<FeeTransferActor>(myself.clone()).await?;
        let fee_transfer_via_coinbase_actor = spawn_actor::<FeeTransferViaCoinbaseActor>(myself.clone()).await?;
        let ledger_actor = spawn_actor::<LedgerActor>(myself.clone()).await?;
        let mainnet_block_parser_actor = spawn_actor::<MainnetBlockParserActor>(myself.clone()).await?;
        let new_account_actor = spawn_actor::<NewAccountActor>(myself.clone()).await?;
        let pcb_path_actor = spawn_actor::<PCBBlockPathActor>(myself.clone()).await?;
        let snark_canonicity_summary_actor = spawn_actor::<SnarkCanonicitySummaryActor>(myself.clone()).await?;
        let snark_summary_persistence_actor = spawn_actor::<SnarkSummaryPersistenceActor>(myself.clone()).await?;
        let snark_work_actor = spawn_actor::<SnarkWorkSummaryActor>(myself.clone()).await?;
        let transition_frontier_actor = spawn_actor::<TransitionFrontierActor>(myself.clone()).await?;
        let user_command_log_actor = spawn_actor::<UserCommandLogActor>(myself.clone()).await?;

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

    async fn handle(&self, _myself: ActorRef<Self::Msg>, msg: Self::Msg, _state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match msg {
            Event::GenesisBlock(_) => {
                self.block_ancestor_actor.cast(msg)?;
            }
            Event::PrecomputedBlockPath(_) => {
                self.pcb_path_actor.cast(msg)?;
            }
            Event::BerkeleyBlockPath(_) => {
                self.berkeley_block_parser_actor.cast(msg)?;
            }
            Event::MainnetBlockPath(_) => {
                self.mainnet_block_parser_actor.cast(msg)?;
            }
            Event::BlockAncestor(_) => {
                self.blockchain_tree_builder_actor.cast(msg)?;
            }
            Event::BerkeleyBlock(_) => {
                self.block_canonicity_actor.cast(msg)?;
            }
            Event::MainnetBlock(_) => {
                self.block_canonicity_actor.cast(msg.clone())?;
                self.block_log_actor.cast(msg.clone())?;
                self.user_command_log_actor.cast(msg.clone())?;
                self.fee_transfer_actor.cast(msg.clone())?;
                self.fee_transfer_via_coinbase_actor.cast(msg.clone())?;
                self.snark_work_actor.cast(msg.clone())?;
                self.new_account_actor.cast(msg.clone())?;
            }
            Event::NewBlock(_) => {
                self.block_log_actor.cast(msg)?;
            }
            Event::BlockCanonicityUpdate(_) => {
                self.best_block_actor.cast(msg.clone())?;
                self.canonical_block_log_actor.cast(msg.clone())?;
                self.canonical_internal_command_log_actor.cast(msg.clone())?;
                self.canonical_user_command_log_actor.cast(msg.clone())?;
            }
            Event::BlockLog(_) => {
                self.canonical_block_log_actor.cast(msg)?;
            }
            Event::UserCommandLog(_) => {
                self.canonical_user_command_log_actor.cast(msg)?;
            }
            Event::InternalCommandLog(_) => {
                self.canonical_internal_command_log_actor.cast(msg)?;
            }
            Event::CanonicalBlockLog(_) => {
                self.canonical_block_log_persistence_actor.cast(msg)?;
            }
            Event::CanonicalUserCommandLog(_) => {
                self.canonical_user_command_persistence_actor.cast(msg)?;
            }
            Event::CanonicalInternalCommandLog(_) => {
                self.canonical_internal_command_log_persistence_actor.cast(msg)?;
            }
            Event::DoubleEntryTransaction(_) => {
                self.ledger_actor.cast(msg)?;
            }
            Event::NewAccount(_) => {
                self.ledger_actor.cast(msg)?;
            }
            Event::BlockConfirmation(_) => {
                self.block_confirmations_actor.cast(msg)?;
            }
            Event::PreExistingAccount(_) => {
                self.ledger_actor.cast(msg)?;
            }
            Event::ActorHeight(_) => {
                self.transition_frontier_actor.cast(msg)?;
            }
            _ => {}
        }
        Ok(())
    }
}

async fn spawn_actor<A>(parent: A::Arguments) -> Result<ActorRef<Event>, ActorProcessingErr>
where
    A: Actor<Msg = Event> + Default + 'static,
{
    use std::any::type_name;
    let actor_name = type_name::<A>().to_string();
    let (actor_ref, _) = Actor::spawn(Some(actor_name), A::default(), parent).await?;
    Ok(actor_ref)
}

pub fn error_unhandled_event(msg: &Event) -> ActorProcessingErr {
    ActorProcessingErr::from(format!("Unhandled event: {}", std::any::type_name_of_val::<Event>(&msg)))
}

#[tokio::test]
async fn test_process_blocks_dir_with_mainnet_blocks() -> anyhow::Result<()> {
    use crate::event_sourcing::{events::Event, payloads::*, sourcing::*};
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

    let blocks_dir = PathBuf::from_str("./src/event_sourcing/test_data/100_mainnet_blocks").expect("Directory with mainnet blocks should exist");
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
    use crate::event_sourcing::{events::Event, payloads::BlockCanonicityUpdatePayload, sourcing::*};
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

    let blocks_dir = PathBuf::from_str("./src/event_sourcing/test_data/10_mainnet_blocks").expect("Directory with mainnet blocks should exist");
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
