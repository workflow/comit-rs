import * as bcoin from "bcoin";
import { BitcoinNodeConfig } from "../../lib/bitcoin";
import { Asset } from "../cnd_http_api";
import { Wallet } from "./index";

export class BitcoinWallet implements Wallet {
    public static async newInstance(name: string, config: BitcoinNodeConfig) {
        const regtestOptions = JSON.parse(
            JSON.stringify(bcoin.networks.regtest)
        );
        regtestOptions.port = config.p2pPort;
        regtestOptions.rpcPor = config.rpcPort;

        const regtest = bcoin.Network.create(regtestOptions);

        const walletDB = new bcoin.wallet.WalletDB({
            network: regtest,
            memory: true,
        });

        await walletDB.open();

        const privateKey = bcoin.HDPrivateKey.generate()
            .derive(44, true)
            .derive(0, true)
            .derive(0, true);

        const wallet = walletDB.create({
            name,
            accountKey: privateKey.xpubkey(regtest.type),
        });

        return new BitcoinWallet(privateKey, wallet);
    }

    private constructor(
        private readonly privateKey: any,
        private readonly wallet: any
    ) {}

    public mint(asset: Asset): Promise<void> {
        throw new Error("not yet implemented");
    }

    public async newReceiveAddress(): Promise<string> {
        return this.wallet.createReceive(0);
    }
}
