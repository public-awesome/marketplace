use cosmwasm_std::Addr;
pub struct MarketAccounts {
    pub owner: Addr,
    pub bidder: Addr,
    pub creator: Addr,
}
