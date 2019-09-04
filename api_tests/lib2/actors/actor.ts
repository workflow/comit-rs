import { parseEther } from "ethers/utils";
import { Logger } from "log4js";
import { IRestResponse, RestClient } from "typed-rest-client/RestClient";
import URI from "urijs";
import {
    Action,
    EmbeddedRepresentationSubEntity,
    Entity,
} from "../../gen/siren";
import { sleep } from "../../lib/util";
import {
    ActionKind,
    Asset,
    AssetKind,
    CreateSwapRequestPayload,
    Ledger,
} from "../cnd_http_api";
import rejectAfter from "../reject_after";
import { Wallets } from "../wallets";
import { Actors } from "./index";

export class Actor {
    public actors: Actors;
    public wallets: Wallets;

    private readonly logger: Logger;
    private readonly restClient: RestClient;
    private mostRecentSwap: string;

    constructor(
        loggerFactory: () => Logger,
        private readonly name: string,
        cndEndpoint: string
    ) {
        this.logger = loggerFactory();
        this.logger.addContext("role", name);

        this.logger.info("Created new actor at %s", cndEndpoint);
        this.restClient = new RestClient("cnd-test-suite", cndEndpoint);

        // Initialize with default dependencies so that we don't get type check errors but fail at runtime
        this.actors = new Actors(new Map<string, Actor>());
        this.wallets = new Wallets({});
    }

    public async getPeerId(): Promise<string> {
        return Promise.reject("not yet implemented");
    }

    public async sendRequest(
        alphaAssetKind: AssetKind,
        betaAssetKind: AssetKind
    ): Promise<IRestResponse<void>> {
        // By default, we will send the swap request to bob
        const to = this.actors.bob;

        this.logger.debug("Sending swap request to %s", to.name);

        const alphaLedger = defaultLedgerDescriptionForAsset(alphaAssetKind);
        const alphaAsset = defaultAssetDescriptionForAsset(alphaAssetKind);

        this.logger.debug(
            "Derived %o from asset %s",
            alphaLedger,
            alphaAssetKind
        );
        this.logger.debug(
            "Derived %o from asset %s",
            alphaAsset,
            alphaAssetKind
        );

        const betaLedger = defaultLedgerDescriptionForAsset(betaAssetKind);
        const betaAsset = defaultAssetDescriptionForAsset(betaAssetKind);

        this.logger.debug(
            "Derived %o from asset %s",
            betaLedger,
            betaAssetKind
        );
        this.logger.debug("Derived %o from asset %s", betaAsset, betaAssetKind);

        this.wallets.initializeForLedger(alphaLedger.name);
        this.wallets.initializeForLedger(betaLedger.name);

        to.wallets.initializeForLedger(alphaLedger.name);
        to.wallets.initializeForLedger(betaLedger.name);

        this.wallets.getWalletForLedger(alphaLedger.name).mint(alphaAsset);
        to.wallets.getWalletForLedger(betaLedger.name).mint(betaAsset);

        const payload: CreateSwapRequestPayload = {
            alpha_ledger: alphaLedger,
            beta_ledger: betaLedger,
            alpha_asset: alphaAsset,
            beta_asset: betaAsset,
            alpha_expiry: defaultExpiryTimes().alpha_expiry,
            beta_expiry: defaultExpiryTimes().beta_expiry,
            peer: await to.getPeerId(),
            ...(await this.additionalIdentities(alphaAssetKind, betaAssetKind)),
        };

        const response = await this.restClient.create<void>(
            "/swaps/rfc003",
            payload
        );
        const headers = response.headers as any;
        const location = headers.Location;

        this.logger.debug("Created new swap at %s", location);

        if (location) {
            this.mostRecentSwap = location;

            // Inform the other party about the swap that we sent

            const swap = await this.restClient.get<Entity>(location);
            // We don't yet have a shared identifier between Alice and Bob.
            // (Ab)-use the secret-hash for now to uniquely identify the same swap on both sides.
            const secretHash =
                swap.result.properties.state.communication.secret_hash;

            this.logger.debug(
                "Swap %s has secret hash %s",
                location,
                secretHash
            );

            to.mostRecentSwap = await to.findSwapWithSecretHash(secretHash);
        }

        return Promise.resolve(response);
    }

    public async accept(): Promise<void> {
        const timeout = 3000;

        this.logger.debug(
            "Accepting swap request %s with timeout of %dms",
            this.mostRecentSwap,
            timeout
        );

        const swapResponse = await Promise.race([
            rejectAfter<IRestResponse<Entity>>(timeout),
            this.pollMostRecentSwapUntil(hasAction(ActionKind.Accept)),
        ]);

        const acceptAction = swapResponse.result.actions.find(
            action => action.name === ActionKind.Accept
        );

        const request = await this.buildRequestFromAction(acceptAction);

        await this.restClient.client.request(
            request.method,
            request.url,
            JSON.stringify(request.body),
            {}
        );
    }

    public async fund(): Promise<void> {
        return Promise.reject("not yet implemented");
    }

    public async redeem(): Promise<void> {
        return Promise.reject("not yet implemented");
    }

    public async assertSwapped(): Promise<void> {
        return Promise.reject("not yet implemented");
    }

    private async additionalIdentities(alphaAsset: string, betaAsset: string) {
        if (alphaAsset === "bitcoin" && betaAsset === "ether") {
            return {
                beta_ledger_redeem_identity: await this.wallets.ethereum.account(),
            };
        }

        return {};
    }

    private async pollMostRecentSwapUntil(
        predicate: (body: Entity) => boolean
    ): Promise<IRestResponse<Entity>> {
        const response = await this.restClient.get<Entity>(this.mostRecentSwap);

        if (predicate(response.result)) {
            return Promise.resolve(response);
        } else {
            await sleep(500);
            return this.pollMostRecentSwapUntil(predicate);
        }
    }

    /*
     * Find a swap on a given Actor with the given secretHash
     *
     * This function will recurse until it actually finds a matching swap.
     * Most likely, you want to combine the returned Promise with a timeout to not recurse forever.
     */
    private async findSwapWithSecretHash(secretHash: string): Promise<string> {
        this.logger.debug("Looking for swap with secret hash %s", secretHash);

        const allSwaps = await this.restClient.get<Entity>("/swaps");

        const entities: EmbeddedRepresentationSubEntity[] =
            allSwaps.result.entities || [];

        const allSwapsWithState = await Promise.all(
            entities.map(entity => {
                const selfLink = entity.links.find(link =>
                    link.rel.includes("self")
                );

                return this.restClient.get<Entity>(selfLink.href);
            })
        );

        const matchingSwap = allSwapsWithState
            .map(response => response.result)
            .find(
                entity =>
                    entity.properties.state.communication.secret_hash ===
                    secretHash
            );

        if (matchingSwap) {
            const selfLink = matchingSwap.links.find(link =>
                link.rel.includes("self")
            ).href;

            this.logger.debug(
                "Found swap with secret hash %s as %s",
                secretHash,
                selfLink
            );

            return Promise.resolve(selfLink);
        } else {
            await sleep(500);
            return this.findSwapWithSecretHash(secretHash);
        }
    }

    private async buildRequestFromAction(action: Action) {
        const data: any = {};

        for (const field of action.fields || []) {
            if (
                field.class.some((e: string) => e === "ethereum") &&
                field.class.some((e: string) => e === "address")
            ) {
                const address = await this.wallets.ethereum.account();
                data[field.name] = address;

                this.logger.debug(
                    "Ethereum identity for action %s is %s",
                    action.name,
                    address
                );
            }

            if (
                field.class.some((e: string) => e === "bitcoin") &&
                field.class.some((e: string) => e === "feePerWU")
            ) {
                data[field.name] = 20;
            }

            if (
                field.class.some((e: string) => e === "bitcoin") &&
                field.class.some((e: string) => e === "address")
            ) {
                const address = await this.wallets.bitcoin.newReceiveAddress();

                data[field.name] = address;

                this.logger.debug(
                    "Bitcoin identity for action %s is %s",
                    action.name,
                    address
                );
            }
        }

        const method = action.method || "GET";
        if (method === "GET") {
            return {
                method,
                url: new URI(action.href).query(data).toString(),
                body: {},
            };
        } else {
            if (action.type !== "application/json") {
                throw new Error(
                    "Only application/json is supported for non-GET requests."
                );
            }

            return {
                method,
                url: action.href,
                body: data,
            };
        }
    }
}

function defaultLedgerDescriptionForAsset(asset: AssetKind): Ledger {
    switch (asset) {
        case "bitcoin": {
            return {
                name: "bitcoin",
                network: "regtest",
            };
        }
        case "ether": {
            return {
                name: "ethereum",
                network: "regtest",
            };
        }
    }
}

function defaultAssetDescriptionForAsset(asset: AssetKind): Asset {
    switch (asset) {
        case "bitcoin": {
            return {
                name: "bitcoin",
                quantity: "100000000",
            };
        }
        case "ether": {
            return {
                name: "ether",
                quantity: parseEther("10").toString(),
            };
        }
    }
}

function defaultExpiryTimes() {
    const alphaExpiry = new Date("2080-06-11T23:00:00Z").getTime() / 1000;
    const betaExpiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;

    return {
        alpha_expiry: alphaExpiry,
        beta_expiry: betaExpiry,
    };
}

export function hasAction(actionKind: ActionKind) {
    return (body: Entity) =>
        body.actions.findIndex(candidate => candidate.name === actionKind) !==
        -1;
}
