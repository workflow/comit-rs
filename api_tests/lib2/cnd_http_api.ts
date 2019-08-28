export interface Asset {
    name: string;

    [parameter: string]: string;
}

export interface Ledger {
    name: string;

    [parameter: string]: string;
}

export interface CreateSwapRequestPayload {
    alpha_ledger: Ledger;
    beta_ledger: Ledger;
    alpha_asset: Asset;
    beta_asset: Asset;
    beta_ledger_redeem_identity?: string;
    alpha_ledger_refund_identity?: string;
    alpha_expiry: number;
    beta_expiry: number;
    peer: string;
}

export type LedgerAction =
    | {
          type: "bitcoin-send-amount-to-address";
          payload: { to: string; amount: string; network: string };
      }
    | {
          type: "bitcoin-broadcast-signed-transaction";
          payload: {
              hex: string;
              network: string;
              min_median_block_time?: number;
          };
      }
    | {
          type: "ethereum-deploy-contract";
          payload: {
              data: string;
              amount: string;
              gas_limit: string;
              network: string;
          };
      }
    | {
          type: "ethereum-call-contract";
          payload: {
              contract_address: string;
              data: string;
              gas_limit: string;
              network: string;
              min_block_timestamp?: number;
          };
      };

export enum ActionKind {
    Accept = "accept",
    Decline = "decline",
    Deploy = "deploy",
    Fund = "fund",
    Redeem = "redeem",
    Refund = "refund",
}
