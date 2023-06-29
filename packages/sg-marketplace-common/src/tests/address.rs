use cosmwasm_std::{testing::mock_dependencies, Addr};

use crate::address::{address_or, map_validate};

#[test]
fn try_map_validate() {
    let deps = mock_dependencies();

    let addresses = vec![
        String::from("addr1"),
        String::from("addr2"),
        String::from("addr3"),
        String::from("addr1"),
    ];

    let result = map_validate(&deps.api, &addresses);

    assert_eq!(
        result,
        Ok(vec![
            Addr::unchecked("addr1"),
            Addr::unchecked("addr2"),
            Addr::unchecked("addr3"),
        ])
    )
}

#[test]
fn try_address_or() {
    let first_address = Addr::unchecked("addr1");
    let second_address = Addr::unchecked("addr2");

    let result = address_or(Some(&first_address), &second_address);
    assert_eq!(result, first_address);

    let result = address_or(None, &second_address);
    assert_eq!(result, second_address);
}
