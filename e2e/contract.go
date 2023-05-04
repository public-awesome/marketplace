package e2e_test

import (
	"fmt"
	"io/ioutil"
	"sync"

	wasmtypes "github.com/CosmWasm/wasmd/x/wasm/types"
	sdk "github.com/cosmos/cosmos-sdk/types"
)

type storeCache struct {
	sync.Mutex
	contracts map[string][]byte
}

var contractsCache = storeCache{contracts: make(map[string][]byte)}

func StoreContract(ctx sdk.Context, msgServer wasmtypes.MsgServer, creator string, contract string) (uint64, error) {
	b, err := getContractBytes(contract)
	if err != nil {
		return 0, err
	}
	res, err := msgServer.StoreCode(sdk.WrapSDKContext(ctx), &wasmtypes.MsgStoreCode{
		Sender:       creator,
		WASMByteCode: b,
	})
	if err != nil {
		return 0, err
	}
	return res.CodeID, nil
}

func getContractBytes(contract string) ([]byte, error) {
	contractsCache.Lock()
	bz, found := contractsCache.contracts[contract]
	contractsCache.Unlock()
	if found {
		return bz, nil
	}
	contractsCache.Lock()
	defer contractsCache.Unlock()
	var err error
	bz, err = ioutil.ReadFile(fmt.Sprintf("contracts/%s", contract))
	if err != nil {
		return nil, err
	}
	contractsCache.contracts[contract] = bz
	return bz, nil
}
