import { ethers } from "ethers";
import { JsonRpcProvider } from "ethers/providers";
import { parseUnits } from "ethers/utils";
import { EthereumNodeConfig } from "../../lib/ethereum";
import { Asset } from "../cnd_http_api";
import { Wallet } from "./index";

export class EthereumWallet implements Wallet {
    private readonly client: JsonRpcProvider;
    private readonly inner: ethers.Wallet;
    private readonly parity: ethers.Wallet;

    constructor(config: EthereumNodeConfig) {
        this.client = new JsonRpcProvider(config.rpc_url);
        this.inner = ethers.Wallet.createRandom();
        this.parity = new ethers.Wallet(
            "0x4d5db4107d237df6a3d58ee5f70ae63d73d7658d4026f2eefd2f204c81682cb7",
            this.client
        );
    }

    public async mint(asset: Asset): Promise<void> {
        if (asset.name !== "ether") {
            throw new Error(`Cannot mint asset ${name} with EthereumWallet`);
        }

        const minimumAsset = parseUnits(asset.quantity, "wei");

        await this.parity.sendTransaction({
            to: this.account(),
            value: minimumAsset.mul(2), // make sure we have at least twice as much
            gasLimit: 21000,
        });
    }

    public account(): string {
        return this.inner.address;
    }
}
