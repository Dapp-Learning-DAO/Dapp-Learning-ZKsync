import path from "path";
import { encodePacked, keccak256, parseEther, toHex } from "viem";
import { groth16 } from "snarkjs";
import { buildPoseidon } from "circomlibjs";
import Vkey from "../verification_key.json";
import { ethers } from "ethers";

export function convertCallData(calldata: string) {
  const argv = calldata.replace(/["[\]\s]/g, "").split(",");

  const a = [argv[0], argv[1]];
  const b = [
    [argv[2], argv[3]],
    [argv[4], argv[5]],
  ];
  const c = [argv[6], argv[7]];

  let input = [];
  // const input = [argv[8], argv[9]];
  for (let i = 8; i < argv.length; i++) {
    input.push(argv[i] as never);
  }

  return { a, b, c, input };
}

export const calcProof = async (input: string) => {
  const proveRes = await groth16.fullProve(
    { in: keccak256(toHex(input)) },
    path.join(__dirname, "../datahash_js/datahash.wasm"),
    path.join(__dirname, "../circuit_final.zkey")
  );

  const res = await groth16.verify(
    Vkey,
    proveRes.publicSignals,
    proveRes.proof
  );

  if (res) {
    // console.log("calculateProof verify passed!");

    const proof = convertCallData(
      await groth16.exportSolidityCallData(
        proveRes.proof,
        proveRes.publicSignals
      )
    );

    return {
      proof: proof,
      publicSignals: proveRes.publicSignals,
    };
  } else {
    console.error("calculateProof verify faild.");
    return null;
  }
};

export function hashToken(account) {
  return Buffer.from(
    ethers.solidityPackedKeccak256(["address"], [account]).slice(2),
    "hex"
  );
}

export const calculatePublicSignals = async (input: string) => {
  const poseidon = await buildPoseidon();
  const hash = poseidon.F.toString(poseidon([keccak256(toHex(input))]));
  return toHex(BigInt(hash), { size: 32 });
};
