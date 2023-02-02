/**
* This file was automatically generated by @cosmwasm/ts-codegen@0.24.0.
* DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
* and run the @cosmwasm/ts-codegen generate command to regenerate this file.
*/

import { CosmWasmClient, SigningCosmWasmClient, ExecuteResult } from "@cosmjs/cosmwasm-stargate";
import { StdFee } from "@cosmjs/amino";
import { Uint128, Duration, InstantiateMsg, ExpiryRange, ExecuteMsg, Timestamp, Uint64, SaleType, Coin, QueryMsg, Addr, AskOffset, CollectionOffset, BidOffset, CollectionBidOffset, AsksResponse, Ask, AskCountResponse, HooksResponse, BidResponse, Bid, BidsResponse, CollectionBidResponse, CollectionBid, CollectionsResponse, Decimal, ParamsResponse, SudoParams } from "./Marketplace.types";
export interface MarketplaceReadOnlyInterface {
  contractAddress: string;
  collections: ({
    limit,
    startAfter
  }: {
    limit?: number;
    startAfter?: string;
  }) => Promise<CollectionsResponse>;
  ask: ({
    collection,
    tokenId
  }: {
    collection: string;
    tokenId: number;
  }) => Promise<AsksResponse>;
  asks: ({
    collection,
    includeInactive,
    limit,
    startAfter
  }: {
    collection: string;
    includeInactive?: boolean;
    limit?: number;
    startAfter?: number;
  }) => Promise<AsksResponse>;
  reverseAsks: ({
    collection,
    includeInactive,
    limit,
    startBefore
  }: {
    collection: string;
    includeInactive?: boolean;
    limit?: number;
    startBefore?: number;
  }) => Promise<AsksResponse>;
  asksSortedByPrice: ({
    collection,
    includeInactive,
    limit,
    startAfter
  }: {
    collection: string;
    includeInactive?: boolean;
    limit?: number;
    startAfter?: AskOffset;
  }) => Promise<AsksResponse>;
  reverseAsksSortedByPrice: ({
    collection,
    includeInactive,
    limit,
    startBefore
  }: {
    collection: string;
    includeInactive?: boolean;
    limit?: number;
    startBefore?: AskOffset;
  }) => Promise<AsksResponse>;
  askCount: ({
    collection
  }: {
    collection: string;
  }) => Promise<AskCountResponse>;
  asksBySeller: ({
    includeInactive,
    limit,
    seller,
    startAfter
  }: {
    includeInactive?: boolean;
    limit?: number;
    seller: string;
    startAfter?: CollectionOffset;
  }) => Promise<AsksResponse>;
  bid: ({
    bidder,
    collection,
    tokenId
  }: {
    bidder: string;
    collection: string;
    tokenId: number;
  }) => Promise<BidResponse>;
  bidsByBidder: ({
    bidder,
    limit,
    startAfter
  }: {
    bidder: string;
    limit?: number;
    startAfter?: CollectionOffset;
  }) => Promise<BidsResponse>;
  bidsByBidderSortedByExpiration: ({
    bidder,
    limit,
    startAfter
  }: {
    bidder: string;
    limit?: number;
    startAfter?: CollectionOffset;
  }) => Promise<BidsResponse>;
  bids: ({
    collection,
    limit,
    startAfter,
    tokenId
  }: {
    collection: string;
    limit?: number;
    startAfter?: string;
    tokenId: number;
  }) => Promise<BidsResponse>;
  bidsSortedByPrice: ({
    collection,
    limit,
    startAfter
  }: {
    collection: string;
    limit?: number;
    startAfter?: BidOffset;
  }) => Promise<BidsResponse>;
  reverseBidsSortedByPrice: ({
    collection,
    limit,
    startBefore
  }: {
    collection: string;
    limit?: number;
    startBefore?: BidOffset;
  }) => Promise<BidsResponse>;
  collectionBid: ({
    bidder,
    collection
  }: {
    bidder: string;
    collection: string;
  }) => Promise<CollectionBidResponse>;
  collectionBidsByBidder: ({
    bidder,
    limit,
    startAfter
  }: {
    bidder: string;
    limit?: number;
    startAfter?: CollectionOffset;
  }) => Promise<CollectionBidResponse>;
  collectionBidsByBidderSortedByExpiration: ({
    bidder,
    limit,
    startAfter
  }: {
    bidder: string;
    limit?: number;
    startAfter?: CollectionBidOffset;
  }) => Promise<CollectionBidResponse>;
  collectionBidsSortedByPrice: ({
    collection,
    limit,
    startAfter
  }: {
    collection: string;
    limit?: number;
    startAfter?: CollectionBidOffset;
  }) => Promise<CollectionBidResponse>;
  reverseCollectionBidsSortedByPrice: ({
    collection,
    limit,
    startBefore
  }: {
    collection: string;
    limit?: number;
    startBefore?: CollectionBidOffset;
  }) => Promise<CollectionBidResponse>;
  askHooks: () => Promise<HooksResponse>;
  bidHooks: () => Promise<HooksResponse>;
  saleHooks: () => Promise<HooksResponse>;
  params: () => Promise<ParamsResponse>;
}
export class MarketplaceQueryClient implements MarketplaceReadOnlyInterface {
  client: CosmWasmClient;
  contractAddress: string;

  constructor(client: CosmWasmClient, contractAddress: string) {
    this.client = client;
    this.contractAddress = contractAddress;
    this.collections = this.collections.bind(this);
    this.ask = this.ask.bind(this);
    this.asks = this.asks.bind(this);
    this.reverseAsks = this.reverseAsks.bind(this);
    this.asksSortedByPrice = this.asksSortedByPrice.bind(this);
    this.reverseAsksSortedByPrice = this.reverseAsksSortedByPrice.bind(this);
    this.askCount = this.askCount.bind(this);
    this.asksBySeller = this.asksBySeller.bind(this);
    this.bid = this.bid.bind(this);
    this.bidsByBidder = this.bidsByBidder.bind(this);
    this.bidsByBidderSortedByExpiration = this.bidsByBidderSortedByExpiration.bind(this);
    this.bids = this.bids.bind(this);
    this.bidsSortedByPrice = this.bidsSortedByPrice.bind(this);
    this.reverseBidsSortedByPrice = this.reverseBidsSortedByPrice.bind(this);
    this.collectionBid = this.collectionBid.bind(this);
    this.collectionBidsByBidder = this.collectionBidsByBidder.bind(this);
    this.collectionBidsByBidderSortedByExpiration = this.collectionBidsByBidderSortedByExpiration.bind(this);
    this.collectionBidsSortedByPrice = this.collectionBidsSortedByPrice.bind(this);
    this.reverseCollectionBidsSortedByPrice = this.reverseCollectionBidsSortedByPrice.bind(this);
    this.askHooks = this.askHooks.bind(this);
    this.bidHooks = this.bidHooks.bind(this);
    this.saleHooks = this.saleHooks.bind(this);
    this.params = this.params.bind(this);
  }

  collections = async ({
    limit,
    startAfter
  }: {
    limit?: number;
    startAfter?: string;
  }): Promise<CollectionsResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      collections: {
        limit,
        start_after: startAfter
      }
    });
  };
  ask = async ({
    collection,
    tokenId
  }: {
    collection: string;
    tokenId: number;
  }): Promise<AsksResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      ask: {
        collection,
        token_id: tokenId
      }
    });
  };
  asks = async ({
    collection,
    includeInactive,
    limit,
    startAfter
  }: {
    collection: string;
    includeInactive?: boolean;
    limit?: number;
    startAfter?: number;
  }): Promise<AsksResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      asks: {
        collection,
        include_inactive: includeInactive,
        limit,
        start_after: startAfter
      }
    });
  };
  reverseAsks = async ({
    collection,
    includeInactive,
    limit,
    startBefore
  }: {
    collection: string;
    includeInactive?: boolean;
    limit?: number;
    startBefore?: number;
  }): Promise<AsksResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      reverse_asks: {
        collection,
        include_inactive: includeInactive,
        limit,
        start_before: startBefore
      }
    });
  };
  asksSortedByPrice = async ({
    collection,
    includeInactive,
    limit,
    startAfter
  }: {
    collection: string;
    includeInactive?: boolean;
    limit?: number;
    startAfter?: AskOffset;
  }): Promise<AsksResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      asks_sorted_by_price: {
        collection,
        include_inactive: includeInactive,
        limit,
        start_after: startAfter
      }
    });
  };
  reverseAsksSortedByPrice = async ({
    collection,
    includeInactive,
    limit,
    startBefore
  }: {
    collection: string;
    includeInactive?: boolean;
    limit?: number;
    startBefore?: AskOffset;
  }): Promise<AsksResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      reverse_asks_sorted_by_price: {
        collection,
        include_inactive: includeInactive,
        limit,
        start_before: startBefore
      }
    });
  };
  askCount = async ({
    collection
  }: {
    collection: string;
  }): Promise<AskCountResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      ask_count: {
        collection
      }
    });
  };
  asksBySeller = async ({
    includeInactive,
    limit,
    seller,
    startAfter
  }: {
    includeInactive?: boolean;
    limit?: number;
    seller: string;
    startAfter?: CollectionOffset;
  }): Promise<AsksResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      asks_by_seller: {
        include_inactive: includeInactive,
        limit,
        seller,
        start_after: startAfter
      }
    });
  };
  bid = async ({
    bidder,
    collection,
    tokenId
  }: {
    bidder: string;
    collection: string;
    tokenId: number;
  }): Promise<BidResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      bid: {
        bidder,
        collection,
        token_id: tokenId
      }
    });
  };
  bidsByBidder = async ({
    bidder,
    limit,
    startAfter
  }: {
    bidder: string;
    limit?: number;
    startAfter?: CollectionOffset;
  }): Promise<BidsResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      bids_by_bidder: {
        bidder,
        limit,
        start_after: startAfter
      }
    });
  };
  bidsByBidderSortedByExpiration = async ({
    bidder,
    limit,
    startAfter
  }: {
    bidder: string;
    limit?: number;
    startAfter?: CollectionOffset;
  }): Promise<BidsResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      bids_by_bidder_sorted_by_expiration: {
        bidder,
        limit,
        start_after: startAfter
      }
    });
  };
  bids = async ({
    collection,
    limit,
    startAfter,
    tokenId
  }: {
    collection: string;
    limit?: number;
    startAfter?: string;
    tokenId: number;
  }): Promise<BidsResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      bids: {
        collection,
        limit,
        start_after: startAfter,
        token_id: tokenId
      }
    });
  };
  bidsSortedByPrice = async ({
    collection,
    limit,
    startAfter
  }: {
    collection: string;
    limit?: number;
    startAfter?: BidOffset;
  }): Promise<BidsResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      bids_sorted_by_price: {
        collection,
        limit,
        start_after: startAfter
      }
    });
  };
  reverseBidsSortedByPrice = async ({
    collection,
    limit,
    startBefore
  }: {
    collection: string;
    limit?: number;
    startBefore?: BidOffset;
  }): Promise<BidsResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      reverse_bids_sorted_by_price: {
        collection,
        limit,
        start_before: startBefore
      }
    });
  };
  collectionBid = async ({
    bidder,
    collection
  }: {
    bidder: string;
    collection: string;
  }): Promise<CollectionBidResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      collection_bid: {
        bidder,
        collection
      }
    });
  };
  collectionBidsByBidder = async ({
    bidder,
    limit,
    startAfter
  }: {
    bidder: string;
    limit?: number;
    startAfter?: CollectionOffset;
  }): Promise<CollectionBidResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      collection_bids_by_bidder: {
        bidder,
        limit,
        start_after: startAfter
      }
    });
  };
  collectionBidsByBidderSortedByExpiration = async ({
    bidder,
    limit,
    startAfter
  }: {
    bidder: string;
    limit?: number;
    startAfter?: CollectionBidOffset;
  }): Promise<CollectionBidResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      collection_bids_by_bidder_sorted_by_expiration: {
        bidder,
        limit,
        start_after: startAfter
      }
    });
  };
  collectionBidsSortedByPrice = async ({
    collection,
    limit,
    startAfter
  }: {
    collection: string;
    limit?: number;
    startAfter?: CollectionBidOffset;
  }): Promise<CollectionBidResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      collection_bids_sorted_by_price: {
        collection,
        limit,
        start_after: startAfter
      }
    });
  };
  reverseCollectionBidsSortedByPrice = async ({
    collection,
    limit,
    startBefore
  }: {
    collection: string;
    limit?: number;
    startBefore?: CollectionBidOffset;
  }): Promise<CollectionBidResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      reverse_collection_bids_sorted_by_price: {
        collection,
        limit,
        start_before: startBefore
      }
    });
  };
  askHooks = async (): Promise<HooksResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      ask_hooks: {}
    });
  };
  bidHooks = async (): Promise<HooksResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      bid_hooks: {}
    });
  };
  saleHooks = async (): Promise<HooksResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      sale_hooks: {}
    });
  };
  params = async (): Promise<ParamsResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      params: {}
    });
  };
}
export interface MarketplaceInterface extends MarketplaceReadOnlyInterface {
  contractAddress: string;
  sender: string;
  setAsk: ({
    collection,
    expires,
    findersFeeBps,
    fundsRecipient,
    price,
    reserveFor,
    saleType,
    tokenId
  }: {
    collection: string;
    expires: Timestamp;
    findersFeeBps?: number;
    fundsRecipient?: string;
    price: Coin;
    reserveFor?: string;
    saleType: SaleType;
    tokenId: number;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  removeAsk: ({
    collection,
    tokenId
  }: {
    collection: string;
    tokenId: number;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  updateAskPrice: ({
    collection,
    price,
    tokenId
  }: {
    collection: string;
    price: Coin;
    tokenId: number;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  setBid: ({
    collection,
    expires,
    finder,
    findersFeeBps,
    saleType,
    tokenId
  }: {
    collection: string;
    expires: Timestamp;
    finder?: string;
    findersFeeBps?: number;
    saleType: SaleType;
    tokenId: number;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  removeBid: ({
    collection,
    tokenId
  }: {
    collection: string;
    tokenId: number;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  acceptBid: ({
    bidder,
    collection,
    finder,
    tokenId
  }: {
    bidder: string;
    collection: string;
    finder?: string;
    tokenId: number;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  setCollectionBid: ({
    collection,
    expires,
    findersFeeBps
  }: {
    collection: string;
    expires: Timestamp;
    findersFeeBps?: number;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  removeCollectionBid: ({
    collection
  }: {
    collection: string;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  acceptCollectionBid: ({
    bidder,
    collection,
    finder,
    tokenId
  }: {
    bidder: string;
    collection: string;
    finder?: string;
    tokenId: number;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  syncAsk: ({
    collection,
    tokenId
  }: {
    collection: string;
    tokenId: number;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  removeStaleAsk: ({
    collection,
    tokenId
  }: {
    collection: string;
    tokenId: number;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  removeStaleBid: ({
    bidder,
    collection,
    tokenId
  }: {
    bidder: string;
    collection: string;
    tokenId: number;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  removeStaleCollectionBid: ({
    bidder,
    collection
  }: {
    bidder: string;
    collection: string;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
}
export class MarketplaceClient extends MarketplaceQueryClient implements MarketplaceInterface {
  client: SigningCosmWasmClient;
  sender: string;
  contractAddress: string;

  constructor(client: SigningCosmWasmClient, sender: string, contractAddress: string) {
    super(client, contractAddress);
    this.client = client;
    this.sender = sender;
    this.contractAddress = contractAddress;
    this.setAsk = this.setAsk.bind(this);
    this.removeAsk = this.removeAsk.bind(this);
    this.updateAskPrice = this.updateAskPrice.bind(this);
    this.setBid = this.setBid.bind(this);
    this.removeBid = this.removeBid.bind(this);
    this.acceptBid = this.acceptBid.bind(this);
    this.setCollectionBid = this.setCollectionBid.bind(this);
    this.removeCollectionBid = this.removeCollectionBid.bind(this);
    this.acceptCollectionBid = this.acceptCollectionBid.bind(this);
    this.syncAsk = this.syncAsk.bind(this);
    this.removeStaleAsk = this.removeStaleAsk.bind(this);
    this.removeStaleBid = this.removeStaleBid.bind(this);
    this.removeStaleCollectionBid = this.removeStaleCollectionBid.bind(this);
  }

  setAsk = async ({
    collection,
    expires,
    findersFeeBps,
    fundsRecipient,
    price,
    reserveFor,
    saleType,
    tokenId
  }: {
    collection: string;
    expires: Timestamp;
    findersFeeBps?: number;
    fundsRecipient?: string;
    price: Coin;
    reserveFor?: string;
    saleType: SaleType;
    tokenId: number;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      set_ask: {
        collection,
        expires,
        finders_fee_bps: findersFeeBps,
        funds_recipient: fundsRecipient,
        price,
        reserve_for: reserveFor,
        sale_type: saleType,
        token_id: tokenId
      }
    }, fee, memo, funds);
  };
  removeAsk = async ({
    collection,
    tokenId
  }: {
    collection: string;
    tokenId: number;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      remove_ask: {
        collection,
        token_id: tokenId
      }
    }, fee, memo, funds);
  };
  updateAskPrice = async ({
    collection,
    price,
    tokenId
  }: {
    collection: string;
    price: Coin;
    tokenId: number;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      update_ask_price: {
        collection,
        price,
        token_id: tokenId
      }
    }, fee, memo, funds);
  };
  setBid = async ({
    collection,
    expires,
    finder,
    findersFeeBps,
    saleType,
    tokenId
  }: {
    collection: string;
    expires: Timestamp;
    finder?: string;
    findersFeeBps?: number;
    saleType: SaleType;
    tokenId: number;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      set_bid: {
        collection,
        expires,
        finder,
        finders_fee_bps: findersFeeBps,
        sale_type: saleType,
        token_id: tokenId
      }
    }, fee, memo, funds);
  };
  removeBid = async ({
    collection,
    tokenId
  }: {
    collection: string;
    tokenId: number;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      remove_bid: {
        collection,
        token_id: tokenId
      }
    }, fee, memo, funds);
  };
  acceptBid = async ({
    bidder,
    collection,
    finder,
    tokenId
  }: {
    bidder: string;
    collection: string;
    finder?: string;
    tokenId: number;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      accept_bid: {
        bidder,
        collection,
        finder,
        token_id: tokenId
      }
    }, fee, memo, funds);
  };
  setCollectionBid = async ({
    collection,
    expires,
    findersFeeBps
  }: {
    collection: string;
    expires: Timestamp;
    findersFeeBps?: number;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      set_collection_bid: {
        collection,
        expires,
        finders_fee_bps: findersFeeBps
      }
    }, fee, memo, funds);
  };
  removeCollectionBid = async ({
    collection
  }: {
    collection: string;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      remove_collection_bid: {
        collection
      }
    }, fee, memo, funds);
  };
  acceptCollectionBid = async ({
    bidder,
    collection,
    finder,
    tokenId
  }: {
    bidder: string;
    collection: string;
    finder?: string;
    tokenId: number;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      accept_collection_bid: {
        bidder,
        collection,
        finder,
        token_id: tokenId
      }
    }, fee, memo, funds);
  };
  syncAsk = async ({
    collection,
    tokenId
  }: {
    collection: string;
    tokenId: number;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      sync_ask: {
        collection,
        token_id: tokenId
      }
    }, fee, memo, funds);
  };
  removeStaleAsk = async ({
    collection,
    tokenId
  }: {
    collection: string;
    tokenId: number;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      remove_stale_ask: {
        collection,
        token_id: tokenId
      }
    }, fee, memo, funds);
  };
  removeStaleBid = async ({
    bidder,
    collection,
    tokenId
  }: {
    bidder: string;
    collection: string;
    tokenId: number;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      remove_stale_bid: {
        bidder,
        collection,
        token_id: tokenId
      }
    }, fee, memo, funds);
  };
  removeStaleCollectionBid = async ({
    bidder,
    collection
  }: {
    bidder: string;
    collection: string;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      remove_stale_collection_bid: {
        bidder,
        collection
      }
    }, fee, memo, funds);
  };
}