import { Ask } from "./shared-types";

export interface AsksResponse {
asks: Ask[]
[k: string]: unknown
}
