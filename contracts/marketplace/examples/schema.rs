use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};
use sg_marketplace::msg::{
    AskCountResponse, AsksResponse, BidResponse, BidsResponse, CollectionBidResponse,
    CollectionsResponse, CurrentAskResponse, ExecuteMsg, InstantiateMsg, ParamsResponse, QueryMsg,
    SaleFinalizedHookMsg, SudoMsg,
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
    export_schema(&schema_for!(AsksResponse), &out_dir);
    export_schema(&schema_for!(BidResponse), &out_dir);
    export_schema(&schema_for!(BidsResponse), &out_dir);
    export_schema(&schema_for!(CollectionsResponse), &out_dir);
    export_schema(&schema_for!(CurrentAskResponse), &out_dir);
    export_schema(&schema_for!(AskCountResponse), &out_dir);
    export_schema(&schema_for!(ParamsResponse), &out_dir);
    export_schema(&schema_for!(CollectionBidResponse), &out_dir);

    // cosmwasm-typescript-gen expects the query return type as QueryNameResponse
    // Here we map query resonses to the correct name
    export_schema_with_title(&schema_for!(AsksResponse), &out_dir, "AsksBySellerResponse");
    export_schema_with_title(
        &schema_for!(AsksResponse),
        &out_dir,
        "AsksSortedByPriceResponse",
    );
    export_schema_with_title(&schema_for!(BidsResponse), &out_dir, "BidsByBidderResponse");
    export_schema_with_title(
        &schema_for!(BidsResponse),
        &out_dir,
        "BidsSortedByPriceResponse",
    );
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
        &schema_for!(CollectionsResponse),
        &out_dir,
        "ListedCollectionsResponse",
    );
    export_schema_with_title(
        &schema_for!(SaleFinalizedHookMsg),
        &out_dir,
        "SaleFinalizedHooksResponse",
    );
}
