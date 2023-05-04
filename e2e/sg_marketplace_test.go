package e2e_test

import (
	"encoding/json"

	wasmtypes "github.com/CosmWasm/wasmd/x/wasm/types"
	sdk "github.com/cosmos/cosmos-sdk/types"
)

func (suite *SGTestSuite) TestUnauthorizedMarketplaceInstantiation() {
	ctx, _ := suite.parentCtx.CacheContext()
	creator := suite.accounts[0]
	_, err := InstantiateMarketplace(ctx, suite.msgServer, creator.Address, suite.contracts.marketplace)
	suite.Require().Error(err)
	//suite.Equal("Unauthorized: instantiate wasm contract failed", err.Error())
}

func InstantiateMarketplace(ctx sdk.Context, msgServer wasmtypes.MsgServer, account sdk.AccAddress, codeID uint64) (string, error) {
	instantiate := MarketplaceInstantiateMsg{
		Operators:        []string{"Operator1"},
		TradingFee:       200,
		AskExpiry:        ExpiryRange{Min: 24 * 60 * 60, Max: 180 * 24 * 60 * 60}, // Min 24h Max 6 months
		BidExpiry:        ExpiryRange{Min: 24 * 60 * 60, Max: 180 * 24 * 60 * 60}, // Min 24h Max 6 months
		MaxFindersFee:    1000,
		MinPrice:         5,
		StaleBidDuration: Duration{Height: 100},
		BidRemovalReward: 500,
		ListingFee:       0,
	}
	instantiateMsgRaw, err := json.Marshal(&instantiate)
	if err != nil {
		return "", err
	}

	instantiateRes, err := msgServer.InstantiateContract(sdk.WrapSDKContext(ctx), &wasmtypes.MsgInstantiateContract{
		Sender: account.String(),
		Admin:  account.String(),
		CodeID: codeID,
		Label:  "SG Marketplace",
		Msg:    instantiateMsgRaw,
		Funds:  sdk.NewCoins(),
	})
	if err != nil {
		return "", err
	}
	return instantiateRes.Address, nil
}

type MarketplaceInstantiateMsg struct {
	TradingFee       uint64      `json:"trading_fee_bps"`
	AskExpiry        ExpiryRange `json:"ask_expiry"`
	BidExpiry        ExpiryRange `json:"bid_expiry"`
	Operators        []string    `json:"operators"`
	MaxFindersFee    uint64      `json:"max_finders_fee_bps"`
	MinPrice         uint64      `json:"min_price"`
	StaleBidDuration Duration    `json:"stale_bid_duration"`
	BidRemovalReward uint64      `json:"bid_removal_reward_bps"`
	ListingFee       uint64      `json:"listing_fee"`
}

type ExpiryRange struct {
	Min uint64 `json:"min"`
	Max uint64 `json:"max"`
}

type Duration struct {
	Height uint64 `json:"height"`
}
