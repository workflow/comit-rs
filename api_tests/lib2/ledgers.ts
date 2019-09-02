import BitcoinRpcClient from "bitcoin-core";
import Unsupported from "./unsupported";

// to be removed once we move to the new test runner
import { BitcoinNodeConfig } from "../lib/bitcoin";
import { HarnessGlobal } from "../lib/util";
declare var global: HarnessGlobal;

export interface LedgerDataProvider {
    newIdentity(): Promise<string>;
}

class EthereumLedger implements LedgerDataProvider {
    public newIdentity(): Promise<string> {
        return undefined;
    }
}

class BitcoinLedger implements LedgerDataProvider {
    // @ts-ignore
    private readonly client: BitcoinRpcClient;

    constructor(bitcoinNodeConfig: BitcoinNodeConfig) {
        this.client = new BitcoinRpcClient({
            network: "regtest",
            host: bitcoinNodeConfig.host,
            port: bitcoinNodeConfig.rpcPort,
            username: bitcoinNodeConfig.username,
            password: bitcoinNodeConfig.password,
        });
    }

    public newIdentity(): Promise<string> {
        return undefined;
    }
}

export class NullLedger implements LedgerDataProvider {
    private readonly name: string;

    constructor(name: string) {
        this.name = name;
    }

    public newIdentity(): Promise<string> {
        return this.fail();
    }

    private fail(): Promise<string> {
        return Promise.reject(
            `LedgerDataProvider for ${this.name} has not been initialized/created.`
        );
    }
}

export default async function ledgerDataProvider(
    name: string,
    parameters: any
): Promise<LedgerDataProvider> {
    switch (name) {
        case "ethereum": {
            if (parameters.network && parameters.network !== "regtest") {
                throw new Unsupported(
                    `Network '${parameters.network}' on ledger Ethereum`
                );
            }

            return Promise.resolve(new EthereumLedger());
        }
        case "bitcoin": {
            if (parameters.network && parameters.network !== "regtest") {
                throw new Unsupported(
                    `Network '${parameters.network}' on ledger Bitcoin`
                );
            }

            const bitcoinLedgerConfig = global.ledgerConfigs.bitcoin;

            if (!bitcoinLedgerConfig) {
                throw new Unsupported(
                    `Ledger '${name}' has not been initialized by the harness`
                );
            }

            return Promise.resolve(new BitcoinLedger(bitcoinLedgerConfig));
        }
        default: {
            throw new Unsupported(`Ledger '${name}'`);
        }
    }
}
