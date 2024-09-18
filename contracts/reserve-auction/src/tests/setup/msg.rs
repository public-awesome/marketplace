use cosmwasm_std::Addr;

pub struct Accounts {
    #[allow(dead_code)]
    pub owner: Addr,
    pub bidder: Addr,
    pub creator: Addr,
}
