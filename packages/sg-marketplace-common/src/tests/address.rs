use crate::address::{address_or, map_validate};

use cosmwasm_std::{testing::mock_dependencies, Addr};

#[test]
fn try_map_validate() {
    let mut deps = mock_dependencies();
    deps.api = deps.api.with_prefix("stars");

    let addresses = vec![
        String::from("stars18c6cw8k8k5wxdd6ksmkyc2yjeceth93lczmrqz"),
        String::from("stars15gp36gk6jvfupy8rc4segppa38lhm3helm5f8k"),
        String::from("stars10xwvl28g90ahdu2fm66ccy3ep2cmzgk946klls"),
        String::from("stars18c6cw8k8k5wxdd6ksmkyc2yjeceth93lczmrqz"),
    ];

    let result = map_validate(&deps.api, &addresses);

    assert_eq!(
        result,
        Ok(vec![
            Addr::unchecked("stars10xwvl28g90ahdu2fm66ccy3ep2cmzgk946klls"),
            Addr::unchecked("stars15gp36gk6jvfupy8rc4segppa38lhm3helm5f8k"),
            Addr::unchecked("stars18c6cw8k8k5wxdd6ksmkyc2yjeceth93lczmrqz"),
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
