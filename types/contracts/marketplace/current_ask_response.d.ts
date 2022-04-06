import { Addr, Uint128 } from "./shared-types";

export interface CurrentAskResponse {
ask?: (Ask | null)
[k: string]: unknown
}
export interface Ask {
funds_recipient?: (Addr | null)
price: Uint128
seller: Addr
[k: string]: unknown
}
