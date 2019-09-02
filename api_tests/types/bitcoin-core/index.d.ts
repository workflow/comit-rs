declare module "bitcoin-core" {
    export interface GetBlockchainInfoResponse {
        mediantime: number;
    }

    export interface VerboseRawTransactionResponse {
        vout: Array<{
            scriptPubKey: {
                addresses: string[];
            };
            value: number;
        }>;
    }

    export type HexRawTransactionResponse = string;

    export type GetRawTransactionResponse =
        | null
        | HexRawTransactionResponse
        | VerboseRawTransactionResponse;

    interface CtorArgs {
        network: string;
        port: number;
        host: string;
        username: string;
        password: string;
    }

    export default class BitcoinRpcClient {
        constructor(args: CtorArgs);

        generate(num: number): Promise<string[]>;
        getBlockchainInfo(): Promise<GetBlockchainInfoResponse>;

        getBlockCount(): Promise<number>;

        getRawTransaction(
            txId: string,
            verbose?: boolean,
            blockHash?: string
        ): Promise<GetRawTransactionResponse>;

        sendToAddress(
            address: string,
            amount: number | string
        ): Promise<string>;

        sendRawTransaction(hexString: string): Promise<string>;
    }
}
