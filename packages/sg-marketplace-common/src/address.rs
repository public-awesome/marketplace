use cosmwasm_std::{Addr, Api, StdResult};

pub fn map_validate(api: &dyn Api, addresses: &[String]) -> StdResult<Vec<Addr>> {
    let mut validated_addresses = addresses
        .iter()
        .map(|addr| api.addr_validate(addr))
        .collect::<StdResult<Vec<_>>>()?;
    validated_addresses.sort();
    validated_addresses.dedup();
    Ok(validated_addresses)
}

pub fn address_or(addr: Option<&Addr>, default: &Addr) -> Addr {
    addr.map_or(default.clone(), |addr| addr.clone())
}
