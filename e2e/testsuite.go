package e2e_test

import (
	"testing"
	"time"

	wasmkeeper "github.com/CosmWasm/wasmd/x/wasm/keeper"
	wasmtypes "github.com/CosmWasm/wasmd/x/wasm/types"
	sdk "github.com/cosmos/cosmos-sdk/types"
	"github.com/public-awesome/stargaze/v8/app"
	"github.com/public-awesome/stargaze/v8/testutil/simapp"
	"github.com/stretchr/testify/suite"
	tmproto "github.com/tendermint/tendermint/proto/tendermint/types"
)

const TEST_CHAIN_ID = "stargaze-test-1"
const TEST_CHAIN_DENOM = "ustars"

type SGTestSuite struct {
	suite.Suite
	msgServer wasmtypes.MsgServer
	parentCtx sdk.Context
	app       *app.App
	startTime time.Time

	accounts  []Account
	contracts SGTestContracts
}

type SGTestContracts struct {
	marketplace uint64
}

func (suite *SGTestSuite) SetupSuite() {
	suite.accounts = GetAccounts()
	genAccs, balances := GetAccountsAndBalances(suite.accounts)

	suite.app = simapp.SetupWithGenesisAccounts(suite.T(), suite.T().TempDir(), genAccs, balances...)

	startDateTime, err := time.Parse(time.RFC3339Nano, "2023-01-01T00:00:00Z")
	suite.Require().NoError(err)
	suite.startTime = startDateTime
	suite.parentCtx = suite.app.BaseApp.NewContext(false, tmproto.Header{Height: 1, ChainID: TEST_CHAIN_ID, Time: startDateTime})

	// wasm params
	wasmParams := suite.app.WasmKeeper.GetParams(suite.parentCtx)
	wasmParams.CodeUploadAccess = wasmtypes.AllowEverybody
	suite.app.WasmKeeper.SetParams(suite.parentCtx, wasmParams)
	suite.msgServer = wasmkeeper.NewMsgServerImpl(wasmkeeper.NewDefaultPermissionKeeper(suite.app.WasmKeeper))

	// setup contracts
	suite.contracts.marketplace, err = StoreContract(suite.parentCtx, suite.msgServer, suite.accounts[0].Address.String(), "sg721_base.wasm")
	suite.Require().NoError(err)
	suite.Require().Equal(uint64(1), suite.contracts.marketplace)
}

func TestMarketplaceTestSuite(t *testing.T) {
	suite.Run(t, new(SGTestSuite))
}
