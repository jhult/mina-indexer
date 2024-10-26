#
# Compare MinaSearch staking ledger 79 (jxxZUYeVFQriATHvBCrxmtfwtboFtMbXALVkE4y546MPy597QDD) public key and delegates to MinaExplorer
#

#
# Get (public_key, delegates) from MinaSearch
#
POST https://api.minasearch.com/graphql
```graphql
{
  stakes(
    limit: 500
    sortBy: BALANCE_DESC
    query: { ledgerHash: "jxxZUYeVFQriATHvBCrxmtfwtboFtMbXALVkE4y546MPy597QDD" }
  ) {
    delegate
    public_key
  }
}
```
HTTP 200

[Captures]
mina_search_results: jsonpath "$.data.stakes"

#
# Get (public_key, delegates) from MinaExplorer
#

POST https://graphql.minaexplorer.com
```graphql
{
  stakes(
    limit: 500
    sortBy: BALANCE_DESC
    query: { ledgerHash: "jxxZUYeVFQriATHvBCrxmtfwtboFtMbXALVkE4y546MPy597QDD" }
  ) {
    delegate
    public_key
  }
}
```
HTTP 200

[Asserts]
jsonpath "$.data.stakes" == {{mina_search_results}}
