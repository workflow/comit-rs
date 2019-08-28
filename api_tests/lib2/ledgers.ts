import Unsupported from "./unsupported";

export interface LedgerDataProvider {
    newIdentity(): Promise<string>;
}

class EthereumLedger implements LedgerDataProvider {
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
        default: {
            throw new Unsupported(`Ledger '${name}'`);
        }
    }
}
