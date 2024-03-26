use super::{
    gen::{Transaction, TransactionQueryInput},
    sanitize_json,
};
use crate::protocol::serialization_types::staged_ledger_diff::{
    SignedCommandPayloadBodyJson, StakeDelegationJson, UserCommandJson, UserCommandWithStatusJson,
};

pub(crate) const TX_COLUMN: &str = "tx";

impl TransactionQueryInput {
    pub fn matches(&self, transaction: &Transaction) -> bool {
        let mut matches = true;

        if let Some(ref hash) = self.hash {
            matches = matches && transaction.hash == *hash;
        }
        if let Some(ref fee) = self.fee {
            matches = matches && transaction.fee == *fee;
        }

        if let Some(ref kind) = self.kind {
            matches = matches && transaction.kind == Some(*kind);
        }

        if let Some(canonical) = self.canonical {
            matches = matches && transaction.canonical == canonical;
        }

        if let Some(ref from) = self.from {
            matches = matches && transaction.from == Some(*from);
        }

        if let Some(ref to) = self.to {
            matches = matches && transaction.to == *to;
        }

        if let Some(ref memo) = self.memo {
            matches = matches && transaction.memo == *memo;
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
                    hash: hash.to_string(),
                    from: Some(sanitize_json(sender)),
                    to: sanitize_json(receiver),
                    memo: sanitize_json(payload.common.memo),
                    block_height: height as i64,
                    date_time: Some(datetime),
                    canonical: true,
                    kind: Some(kind.to_string()),
                    token: Some(token_id as i64),
                    nonce: nonce as i64,
                    fee: fee as f64 / 1_000_000_000_f64,
                    amount: amount as f64 / 1_000_000_000_f64,
                    // TODO: Why is this required?
                    failure_reason: String::default(),
                    // TODO: What is this?
                    id: todo!(),
                    is_delegation: None,
                    fee_token: None,
                }
            }
        }
    }
}
