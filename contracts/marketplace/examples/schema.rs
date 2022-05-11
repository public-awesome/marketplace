use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};
use sg_controllers::HooksResponse;
use sg_marketplace::msg::{
    AskCountResponse, AskHookMsg, AskOffset, AskResponse, AsksResponse, BidOffset, BidResponse,
    BidsResponse, CollectionBidOffset, CollectionBidResponse, CollectionOffset,
    CollectionsResponse, ExecuteMsg, InstantiateMsg, ParamsResponse, QueryMsg, SaleHookMsg,
    SudoMsg,
};
use sg_marketplace::MarketplaceContract;
use std::env::current_dir;
use std::fs::create_dir_all;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(MarketplaceContract), &out_dir);
    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(SudoMsg), &out_dir);

    export_schema(&schema_for!(AskCountResponse), &out_dir);
    export_schema(&schema_for!(AskOffset), &out_dir);
    export_schema(&schema_for!(AskResponse), &out_dir);
    export_schema(&schema_for!(AsksResponse), &out_dir);
    export_schema(&schema_for!(BidOffset), &out_dir);
    export_schema(&schema_for!(BidResponse), &out_dir);
    export_schema(&schema_for!(BidsResponse), &out_dir);
    export_schema(&schema_for!(CollectionBidResponse), &out_dir);
    export_schema(&schema_for!(CollectionBidOffset), &out_dir);
    export_schema(&schema_for!(CollectionOffset), &out_dir);
    export_schema(&schema_for!(CollectionsResponse), &out_dir);
    export_schema(&schema_for!(ParamsResponse), &out_dir);

    // cosmwasm-typescript-gen expects the query return type as QueryNameResponse
    // Here we map query resonses to the correct name
    export_schema_with_title(&schema_for!(AsksResponse), &out_dir, "AsksBySellerResponse");
    export_schema_with_title(
        &schema_for!(AskHookMsg),
        &out_dir,
        "AskCreatedHooksResponse",
    );
    export_schema_with_title(
        &schema_for!(AsksResponse),
        &out_dir,
        "AsksSortedByPriceResponse",
    );
    export_schema_with_title(
        &schema_for!(AsksResponse),
        &out_dir,
        "ReverseAsksSortedByPriceResponse",
    );
    export_schema_with_title(&schema_for!(HooksResponse), &out_dir, "AskHooksResponse");

    export_schema_with_title(&schema_for!(BidsResponse), &out_dir, "BidsByBidderResponse");
    export_schema_with_title(
        &schema_for!(BidsResponse),
        &out_dir,
        "BidsSortedByPriceResponse",
    );
    export_schema_with_title(
        &schema_for!(BidsResponse),
        &out_dir,
        "BidsByBidderSortedByExpirationResponse",
    );
    export_schema_with_title(
        &schema_for!(BidsResponse),
        &out_dir,
        "ReverseBidsSortedByPriceResponse",
    );
    export_schema_with_title(&schema_for!(HooksResponse), &out_dir, "BidHooksResponse");

    export_schema_with_title(
        &schema_for!(BidsResponse),
        &out_dir,
        "CollectionBidsByBidderResponse",
    );
    export_schema_with_title(
        &schema_for!(BidsResponse),
        &out_dir,
        "CollectionBidsSortedByPriceResponse",
    );
    export_schema_with_title(
        &schema_for!(BidsResponse),
        &out_dir,
        "CollectionBidsByBidderSortedByExpirationResponse",
    );
    export_schema_with_title(
        &schema_for!(BidsResponse),
        &out_dir,
        "ReverseCollectionBidsSortedByPriceResponse",
    );

    export_schema_with_title(
        &schema_for!(CollectionsResponse),
        &out_dir,
        "ListedCollectionsResponse",
    );
    export_schema_with_title(&schema_for!(SaleHookMsg), &out_dir, "SaleHooksResponse");
}
