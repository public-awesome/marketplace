#[path = "./setup/mock_collection_params.rs"]
#[cfg(test)]
mod mock_collection_params;
#[path = "./setup/setup_accounts_and_block.rs"]
#[cfg(test)]
mod setup_accounts_and_block;
#[path = "./setup/setup_contracts.rs"]
#[cfg(test)]
mod setup_contracts;
#[path = "./setup/setup_minter.rs"]
#[cfg(test)]
mod setup_minter;

#[path = "./tests/auction_tests.rs"]
#[cfg(test)]
mod auction_tests;
#[path = "./tests/fixed_price_tests.rs"]
#[cfg(test)]
mod fixed_price_tests;
#[path = "./tests/multitest.rs"]
#[cfg(test)]
mod multitest;
#[path = "./tests/unit_tests.rs"]
#[cfg(test)]
mod unit_tests;
