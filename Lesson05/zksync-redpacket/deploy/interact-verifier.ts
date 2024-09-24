import * as hre from "hardhat";
import { deployContract, getWallet } from "./utils";
import { ethers, toBigInt } from "ethers";
import { calcProof, calculatePublicSignals, convertCallData } from "../utils";
import contractDeployments from "./zkSync_deployment.json";

// Address of the contract to interact with
// const CONTRACT_ADDRESS = ""; // zksync mainnet

// sepolia SimpleToken address 0xD9a42d80741D4CE4513c16a70032C3B95cbB0CCE

// zero bytes
const ZERO_BYTES32 =
  "0x0000000000000000000000000000000000000000000000000000000000000000";

// An example of a script to interact with the contract
export default async function () {
  const CONTRACT_ADDRESS = contractDeployments.Verifier;
  console.log(`Running script to interact with contract ${CONTRACT_ADDRESS}`);

  // Load compiled contract info
  const contractArtifact = await hre.artifacts.readArtifact("Groth16Verifier");

  const wallet = getWallet();

  // Initialize contract instance for interaction
  const verifier = new ethers.Contract(
    CONTRACT_ADDRESS,
    contractArtifact.abi,
    wallet, // Interact with the contract on behalf of this wallet
  );

  const password = "abcd1234";
  const proofRes = await calcProof(password);
  if (proofRes) {
    const {
      proof: { a, b, c },
      publicSignals,
    } = proofRes;
    const res = await verifier.verifyProof(a, b, c, publicSignals);
    console.log("verifier.verifyProof()", res);
  }
}
