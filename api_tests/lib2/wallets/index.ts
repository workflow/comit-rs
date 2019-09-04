import { HarnessGlobal } from "../../lib/util";
import { Asset } from "../cnd_http_api";
import { BitcoinWallet } from "./bitcoin";
import { EthereumWallet } from "./ethereum";

declare var global: HarnessGlobal;

interface AllWallets {
    bitcoin?: BitcoinWallet;
    ethereum?: EthereumWallet;
}

export interface Wallet {
    mint(asset: Asset): Promise<void>;
}

export class Wallets {
    constructor(private readonly wallets: AllWallets) {}

    get bitcoin(): BitcoinWallet {
        return this.getWalletForLedger("bitcoin");
    }

    get ethereum(): EthereumWallet {
        return this.getWalletForLedger("ethereum");
    }

    public getWalletForLedger<K extends keyof AllWallets>(
        name: K
    ): AllWallets[K] {
        const wallet = this.wallets[name];

        if (!wallet) {
            throw new Error(`Wallet for ${name} was not initialized`);
        }

        return wallet;
    }

    public initializeForLedger<K extends keyof AllWallets>(name: K) {
        switch (name) {
            case "ethereum":
                this.wallets.ethereum = new EthereumWallet(
                    global.ledgerConfigs.ethereum
                );
                break;
            case "bitcoin":
                this.wallets.bitcoin = new BitcoinWallet(
                    global.ledgerConfigs.bitcoin
                );
                break;
        }
    }
}
