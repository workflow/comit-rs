import { createActors } from "../../../lib_sdk/create_actors";

setTimeout(function() {
    describe("cnd can resume swap", function() {
        this.timeout(60000);
        it("while the swap is in progress", async function() {
            const { alice, bob } = await createActors(
                "cnd_can_be_restarted.log"
            );

            await alice.sendRequest();
            await bob.accept();

            await alice.currentSwapIsAccepted();
            await bob.currentSwapIsAccepted();

            await alice.fund();
            await alice.restart();

            await bob.fund();
            await bob.restart();

            await alice.restart();
            await bob.restart();

            await alice.redeem();
            await alice.restart();

            await bob.redeem();
            await bob.restart();

            await alice.assertSwapped();
            await bob.assertSwapped();
        });
    });
    run();
}, 0);
