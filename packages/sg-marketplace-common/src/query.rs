use cosmwasm_schema::cw_serde;
use cosmwasm_std::Order;
use cw_storage_plus::{Bound, PrimaryKey};

/// QueryOptions are used to paginate contract queries
#[cw_serde]
#[derive(Default)]
pub struct QueryOptions<T> {
    /// Whether to sort items in ascending or descending order
    pub descending: Option<bool>,
    /// The key to start the query after
    pub start_after: Option<T>,
    /// The number of items that will be returned
    pub limit: Option<u32>,
}

/// Invoke `unpack_query_options` to build the query options used to paginate contract queries.
pub fn unpack_query_options<'a, T: PrimaryKey<'a>, U>(
    query_options: QueryOptions<U>,
    start_after_fn: Box<dyn FnOnce(U) -> Bound<'a, T>>,
    default_query_limit: u32,
    max_query_limit: u32,
) -> (usize, Order, Option<Bound<'a, T>>, Option<Bound<'a, T>>) {
    let limit = query_options
        .limit
        .unwrap_or(default_query_limit)
        .min(max_query_limit) as usize;

    let mut order = Order::Ascending;
    if let Some(_descending) = query_options.descending {
        if _descending {
            order = Order::Descending;
        }
    };

    let bound = query_options.start_after.map(start_after_fn);

    let (min, max) = match order {
        Order::Ascending => (bound, None),
        Order::Descending => (None, bound),
    };

    (limit, order, min, max)
}
