use cosmwasm_std::Order;
use cw_storage_plus::Bound;

use crate::query::{unpack_query_options, QueryOptions};

#[test]
fn try_unpack_query_options() {
    let (limit, order, min, max) = unpack_query_options(
        QueryOptions {
            limit: Some(20),
            descending: Some(true),
            start_after: Some("test".to_string()),
        },
        Box::new(Bound::<String>::exclusive),
        10u32,
        12u32,
    );

    assert_eq!(limit as u32, 12u32);
    match order {
        Order::Ascending => panic!("Order should be descending"),
        Order::Descending => (),
    }
    if min.is_some() {
        panic!("Min should be None")
    }
    match max {
        Some(_) => {}
        None => panic!("Max should be Some"),
    }
}
