use super::{database::EVENTS, fixed_keys::FixedKeys, DBIterator, IteratorAnchor};
use crate::{
    event::{store::EventStore, IndexerEvent},
    store::{database::INDEXED_U32, IndexerStore},
};
use log::trace;

impl EventStore for IndexerStore {
    fn add_event(&self, event: &IndexerEvent) -> anyhow::Result<u32> {
        let seq_num = self.get_next_seq_num()?;
        trace!("Adding event {seq_num}: {:?}", event);

        if matches!(event, IndexerEvent::WitnessTree(_)) {
            return Ok(seq_num);
        }

        // add event to db
        let key = seq_num.to_be_bytes();
        let value = event;
        self.put(EVENTS, key, event);

        // increment event sequence number
        let next_seq_num = seq_num + 1;
        self.database
            .write(INDEXED_U32, Self::NEXT_EVENT_SEQ_NUM_KEY, next_seq_num)?;

        // return next event sequence number
        Ok(next_seq_num)
    }

    fn get_event(&self, seq_num: u32) -> anyhow::Result<Option<IndexerEvent>> {
        let key = seq_num;

        let event = self.get(EVENTS, key);

        trace!("Getting event {seq_num}: {:?}", event.clone().unwrap());
        Ok(event)
    }

    fn get_next_seq_num(&self) -> anyhow::Result<u32> {
        trace!("Getting next event sequence number");
        Ok(
            if let Some(bytes) = self
                .database
                .get_pinned_cf(&self.events_cf(), Self::NEXT_EVENT_SEQ_NUM_KEY)?
            {
                serde_json::from_slice(&bytes)?
            } else {
                0
            },
        )
    }

    fn get_event_log(&self) -> anyhow::Result<Vec<IndexerEvent>> {
        trace!("Getting event log");

        let mut events = vec![];
        for n in 0..self.get_next_seq_num()? {
            if let Some(event) = self.get_event(n)? {
                events.push(event);
            }
        }
        Ok(events)
    }

    /// Key: sequence number (4 BE bytes)
    /// Value: event (serialized with [serde_json::to_vec])
    fn event_log_iterator(&self, mode: IteratorAnchor) -> DBIterator<u32, IndexerEvent> {
        self.database.iterator(EVENTS)
    }
}
