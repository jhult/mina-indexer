use super::{
    gen::{Transaction, TransactionQueryInput},
    sanitize_json,
};
use crate::protocol::serialization_types::staged_ledger_diff::{
    SignedCommandPayloadBodyJson, StakeDelegationJson, UserCommandJson, UserCommandWithStatusJson,
};
use chrono::Utc;

pub(crate) const TX_COLUMN: &str = "tx";

impl TransactionQueryInput {
    pub fn matches(&self, transaction: &Transaction) -> bool {
        let mut matches = true;

        if let Some(ref hash) = self.hash {
            matches = matches && transaction.hash == Some(*hash);
        }
        if let Some(ref fee) = self.fee {
            matches = matches && transaction.fee == Some(*fee);
        }

        if let Some(ref kind) = self.kind {
            matches = matches && transaction.kind == Some(*kind);
        }

        if let Some(canonical) = self.canonical {
            matches = matches && transaction.canonical == Some(canonical);
        }

        if let Some(ref from) = self.from {
            matches = matches && transaction.from == Some(*from);
        }

        if let Some(ref to) = self.to {
            matches = matches && transaction.to == Some(*to);
        }

        if let Some(ref memo) = self.memo {
            matches = matches && transaction.memo == Some(*memo);
        }

        if let Some(ref query) = self.and {
            matches = matches && query.iter().all(|and| and.matches(transaction));
        }

        if let Some(ref query) = self.or {
            if !query.is_empty() {
                matches = matches && query.iter().any(|or| or.matches(transaction));
            }
        }

        if let Some(datetime_gte) = self.datetime_gte {
            matches = matches && transaction.date_time >= Some(datetime_gte);
        }

        if let Some(datetime_lte) = self.datetime_lte {
            matches = matches && transaction.date_time <= Some(datetime_lte);
        }

        matches
    }
}

impl Transaction {
    pub fn from_cmd(
        cmd: UserCommandWithStatusJson,
        height: i32,
        timestamp: u64,
        hash: &str,
    ) -> Self {
        match cmd.data {
            UserCommandJson::SignedCommand(signed_cmd) => {
                let payload = signed_cmd.payload;
                let token = payload.common.fee_token.0;
                let nonce = payload.common.nonce.0;
                let fee = payload.common.fee.0;
                let (sender, receiver, kind, token_id, amount) = {
                    match payload.body {
                        SignedCommandPayloadBodyJson::PaymentPayload(payload) => (
                            payload.source_pk,
                            payload.receiver_pk,
                            "PAYMENT",
                            token,
                            payload.amount.0,
                        ),
                        SignedCommandPayloadBodyJson::StakeDelegation(payload) => {
                            let StakeDelegationJson::SetDelegate {
                                delegator,
                                new_delegate,
                            } = payload;
                            (delegator, new_delegate, "STAKE_DELEGATION", token, 0)
                        }
                    }
                };

                // TODO: proper incoming format??
                let datetime = chrono::DateTime::<Utc>::from_timestamp_millis(timestamp as i64);

                Self {
                    hash: Some(hash.to_string()),
                    from: Some(sanitize_json(sender)),
                    to: Some(sanitize_json(receiver)),
                    memo: Some(sanitize_json(payload.common.memo)),
                    block_height: Some(height),
                    date_time: Some(datetime),
                    canonical: Some(true),
                    kind: Some(&kind),
                    token: Some(token_id),
                    nonce: Some(nonce),
                    fee: Some(fee as f64 / 1_000_000_000_f64),
                    amount: Some(amount as f64 / 1_000_000_000_f64),
                    failure_reason: todo!(),
                    fee_token: todo!(),
                    id: todo!(),
                    is_delegation: todo!(),
                }
            }
        }
    }
}
