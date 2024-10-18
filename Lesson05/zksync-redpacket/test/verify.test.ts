import { expect } from "chai";
import { getWallet, deployContract, LOCAL_RICH_WALLETS } from "../deploy/utils";
import { calcProof, calculatePublicSignals } from "../utils";
import { toHex } from "viem";


describe("Verify.sol", function () {
  it("Should snark proof correct.", async function () {
    const wallet = getWallet(LOCAL_RICH_WALLETS[0].privateKey);

    const verifier = await deployContract("Groth16Verifier", [], {
      wallet,
      silent: true,
    });

    const password = "This is a correct password";
    const proofRes = await calcProof(password);
    const hashLock = await calculatePublicSignals(password);
    if (proofRes) {
      const {
        proof: { a, b, c },
        publicSignals,
      } = proofRes;
      const res = await verifier.verifyProof(a, b, c, publicSignals);
      expect(res).to.be.eq(true);
      expect(toHex(BigInt(publicSignals[0]))).to.equal(hashLock);
    }
  });
});
