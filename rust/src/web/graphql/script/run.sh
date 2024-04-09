#!/usr/bin/env bash
# https://sharats.me/posts/shell-script-best-practices/

set -o errexit
set -o nounset
set -o pipefail
if [[ "${TRACE-0}" == "1" ]]; then
    set -o xtrace
fi

if [[ "${1-}" =~ ^-*h(elp)?$ ]]; then
    echo 'Usage:
./run.sh download
    Downloads GraphQL schema and queries
./run.sh generate
    Generates Rust files
'
    exit
fi

cd "$(dirname "$0")"

readonly GH="https://raw.githubusercontent.com"
readonly GH_GRANOLA="$GH/Granola-Team"
readonly GH_EXPLORER_GRAPHQL="$GH_GRANOLA/mina-block-explorer/main/graphql"
readonly EXT="graphql"
readonly SCHEMA_FOLDER="../schema"
readonly SCHEMA_FILE="$SCHEMA_FOLDER/mina-explorer.$EXT"
readonly CONFIG_FILE="config.toml"

download() {
    curl --output "$SCHEMA_FILE" "$GH_EXPLORER_GRAPHQL/schemas/mina-explorer.$EXT"
    cat "$SCHEMA_FOLDER/schema_base.graphql" >> "$SCHEMA_FILE"
}

generate() {
    async-graphql-reverse --input-schema "$SCHEMA_FILE" --output-dir ../gen --config "$CONFIG_FILE" schema
    cargo +nightly fmt --all
}

main() {
    if [[ "${1-}" == "download" ]]; then
        download
    elif [[ "${1-}" == "generate" ]]; then
        generate
    else
        local readonly help="$(./run.sh --help)"
        echo "$help"
    fi
}

main "$@"
