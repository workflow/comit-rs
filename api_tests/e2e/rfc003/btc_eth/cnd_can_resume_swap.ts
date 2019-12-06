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

            // This does not fully test the fact that we can get a transaction
            // that happend in a block we missed.  To test this suggest:

            // stop alice's node here
            await bob.fund();
            // re-tart alice's node again here

            await alice.restart();
            await bob.restart();

            await alice.redeem();
            await bob.redeem();

            await alice.assertSwapped();
            await bob.assertSwapped();
        });
    });
    run();
}, 0);
