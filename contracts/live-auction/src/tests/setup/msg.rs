use cosmwasm_std::Addr;

pub struct Accounts {
    pub owner: Addr,
    pub bidder: Addr,
    pub creator: Addr,
}
