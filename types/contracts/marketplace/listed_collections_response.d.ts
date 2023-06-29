import { Addr } from "./shared-types";

export interface ListedCollectionsResponse {
collections: Addr[]
[k: string]: unknown
}
