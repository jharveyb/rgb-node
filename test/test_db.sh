#!/bin/bash
rm -f $DATABASE_URL/assets/assets*

cd db/cache/
diesel database setup --database-url $DATABASE_URL/assets/assets.db --config-file ./diesel.toml
diesel migration run --database-url $DATABASE_URL/assets/assets.db --config-file ./diesel.toml

cd ../..

cargo test test_sqlite_create_tables

cargo test test_sqlite_asset_cache

cargo test test_sqlite_mappings

cargo test test_filecache_mappings
