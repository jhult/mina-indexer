use super::{
    date_time_to_scalar, db,
    gen::{
        DateTime, Query, Transaction, TransactionBlock, TransactionQueryInput, TransactionReceiver,
        TransactionSortByInput,
    },
    DataSource,
};
use crate::{
    block::BlockHash,
    command::signed::{SignedCommand, SignedCommandWithData},
    ledger::public_key::PublicKey,
    protocol::serialization_types::staged_ledger_diff::{
        SignedCommandPayloadBody, StakeDelegation,
    },
    store::{
        user_commands_iterator, user_commands_iterator_signed_command,
        user_commands_iterator_txn_hash,
    },
};
use async_graphql::{Context, Result};

impl DataSource {
    pub(crate) async fn query_transactions(
        &self,
        ctx: &Context<'_>,
        _: &Query,
        input: TransactionQueryInput,
        limit: Option<i64>,
        _sort_by: TransactionSortByInput,
    ) -> Result<Vec<Option<Transaction>>> {
        let db = db(ctx);
        let limit = limit.unwrap_or(100);
        let limit_idx = limit as usize;

        let mut transactions: Vec<Option<Transaction>> = Vec::new();

        let iter = user_commands_iterator(db);

        for entry in iter {
            let txn_hash = user_commands_iterator_txn_hash(&entry)?;

            if let Some(__) = input.hash.to_owned() {
                if txn_hash != __ {
                    continue;
                }
            }

            let cmd = user_commands_iterator_signed_command(&entry)?;

            let block_state_hash = cmd.state_hash.to_owned();
            let block_date_time = date_time_to_scalar(cmd.date_time as i64);

            let transaction = Transaction::from_cmd(cmd, block_date_time, &block_state_hash);

            // Only add transactions that satisfy the query
            if input.matches(&transaction) {
                transactions.push(Some(transaction));
            };

            // Early break if the transactions reach the query limit
            if transactions.len() >= limit_idx {
                break;
            }
        }

        // match sort_by {
        //     TransactionSortByInput::NonceAsc => {
        //         transactions.sort_by(|Some(a), Some(b)| a.nonce.cmp(&b.nonce))
        //     }
        //     _ => transactions.sort_by(|Some(a), Some(b)| b.nonce.cmp(&a.nonce)),
        // }

        Ok(transactions)
    }

    pub(crate) async fn query_transaction(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _input: TransactionQueryInput,
    ) -> Result<Option<Transaction>> {
        todo!()
    }
}

impl TransactionQueryInput {
    pub fn matches(&self, transaction: &Transaction) -> bool {
        let mut matches = true;

        if let Some(hash) = &self.hash {
            matches = matches && &transaction.hash == hash;
        }
        if let Some(fee) = self.fee {
            matches = matches && transaction.fee == fee;
        }

        if self.kind.is_some() {
            matches = matches && transaction.kind == self.kind;
        }

        if let Some(canonical) = self.canonical {
            matches = matches && transaction.canonical == canonical;
        }

        if self.from.is_some() {
            matches = matches && transaction.from == self.from;
        }

        if let Some(to) = &self.to {
            matches = matches && &transaction.to == to;
        }

        if let Some(memo) = &self.memo {
            matches = matches && &transaction.memo == memo;
        }

        if let Some(query) = &self.and {
            matches = matches && query.iter().all(|and| and.matches(transaction));
        }

        if let Some(query) = &self.or {
            if !query.is_empty() {
                matches = matches && query.iter().any(|or| or.matches(transaction));
            }
        }

        if let Some(__) = &self.date_time_gte {
            matches = matches && transaction.block.date_time >= *__;
        }

        if let Some(__) = &self.date_time_lte {
            matches = matches && transaction.block.date_time <= *__;
        }

        // TODO: implement matches for all the other optional vars

        matches
    }
}

impl Transaction {
    pub fn from_cmd(
        cmd: SignedCommandWithData,
        block_date_time: DateTime,
        block_state_hash: &BlockHash,
    ) -> Self {
        match cmd.command {
            SignedCommand(signed_cmd) => {
                let payload = signed_cmd.t.t.payload;
                let token = payload.t.t.common.t.t.t.fee_token.t.t.t;
                let nonce = payload.t.t.common.t.t.t.nonce.t.t;
                let fee = payload.t.t.common.t.t.t.fee.t.t;
                let (sender, receiver, kind, token_id, amount) = {
                    match payload.t.t.body.t.t {
                        SignedCommandPayloadBody::PaymentPayload(payload) => (
                            payload.t.t.source_pk,
                            payload.t.t.receiver_pk,
                            "PAYMENT",
                            token,
                            payload.t.t.amount.t.t,
                        ),
                        SignedCommandPayloadBody::StakeDelegation(payload) => {
                            let StakeDelegation::SetDelegate {
                                delegator,
                                new_delegate,
                            } = payload.t;
                            (delegator, new_delegate, "STAKE_DELEGATION", token, 0)
                        }
                    }
                };

                let receiver = PublicKey::from(receiver).0;
                let mut memo = String::from_utf8(payload.t.t.common.t.t.t.memo.t.0).unwrap();
                // ignore memos with nonsense unicode
                if memo.starts_with("\u{0001}") {
                    memo = String::new();
                };

                Self {
                    hash: cmd.tx_hash,
                    from: Some(PublicKey::from(sender).0),
                    to: receiver.to_owned(),
                    receiver: TransactionReceiver {
                        public_key: receiver,
                    },
                    memo,
                    block_height: cmd.blockchain_length as i64,
                    block: TransactionBlock {
                        date_time: block_date_time,
                        state_hash: block_state_hash.0.to_owned(),
                    },
                    // TODO: always true ??
                    canonical: true,
                    kind: Some(kind.to_string()),
                    token: Some(token_id as i64),
                    nonce: nonce as i64,
                    fee: fee as f64 / 1_000_000_000_f64,
                    amount: amount as f64 / 1_000_000_000_f64,
                    // TODO: Why is this required?
                    failure_reason: String::new(),
                    // TODO: What is this?
                    id: String::from("Not yet implemented"),
                }
            }
        }
    }
}
