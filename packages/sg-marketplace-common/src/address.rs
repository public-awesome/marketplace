use cosmwasm_std::{Addr, Api, StdResult};

/// Invoke `map_validate` to validate and dedupe a list of string addresses.
pub fn map_validate(api: &dyn Api, addresses: &[String]) -> StdResult<Vec<Addr>> {
    let mut validated_addresses = addresses
        .iter()
        .map(|addr| api.addr_validate(addr))
        .collect::<StdResult<Vec<_>>>()?;
    validated_addresses.sort();
    validated_addresses.dedup();
    Ok(validated_addresses)
}

/// Invoke `address_or` to return the address if it exists, otherwise return the default address.
pub fn address_or(addr: Option<&Addr>, default: &Addr) -> Addr {
    addr.map_or(default.clone(), |addr| addr.clone())
}
