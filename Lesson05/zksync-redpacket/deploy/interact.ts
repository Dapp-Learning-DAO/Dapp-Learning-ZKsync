import * as hre from "hardhat";
import { deployContract, getWallet } from "./utils";
import {
  EventLog,
  Log,
  MaxUint256,
  ethers,
  formatUnits,
  parseUnits,
} from "ethers";
import MerkleTree from "merkletreejs";
import { encodePacked, keccak256, parseEther, toHex } from "viem";
import contractDeployments from "./zkSync_deployment.json";
import { calcProof, calculatePublicSignals, convertCallData } from "../utils";

function hashToken(account: `0x${string}`) {
  return Buffer.from(
    keccak256(encodePacked(["address"], [account])).slice(2),
    "hex",
  );
}

// zero bytes
const ZERO_BYTES32 =
  "0x0000000000000000000000000000000000000000000000000000000000000000";
const ERC20_ADDRESS = "0x264d10475eF47cFABdD3A0592d285ac612A4586D"; // Test REC20 DLD

const tokenAmount = parseUnits("10", 18);
const password = "abcd1234";

export default async function () {
  const CONTRACT_ADDRESS = contractDeployments.Redpacket;
  console.log(`Running script to interact with contract ${CONTRACT_ADDRESS}`);

  // Load compiled contract info
  const contractArtifact = await hre.artifacts.readArtifact("HappyRedPacket");

  const wallet = getWallet();

  // Initialize contract instance for interaction
  const redPacket = new ethers.Contract(
    CONTRACT_ADDRESS,
    contractArtifact.abi,
    wallet, // Interact with the contract on behalf of this wallet
  );
  const redPacketAddress = await redPacket.getAddress();

  const testToken = new ethers.Contract(
    ERC20_ADDRESS,
    (await hre.artifacts.readArtifact("SimpleToken")).abi,
    wallet,
  );
  const testTokenAddress = await testToken.getAddress();

  const allowanceRes = await testToken.allowance(
    wallet.address,
    redPacketAddress,
  );
  console.log(`allowance: ${formatUnits(allowanceRes, 18)}`);

  if (allowanceRes < tokenAmount) {
    let approveRes = await testToken.approve(redPacketAddress, MaxUint256);
    console.log(`Approve tx hash`, approveRes.hash);
  }

  // Run contract read function
  console.log("Redpacket Nonce:", await redPacket.nonce());

  const claimerList = [
    wallet.address,
    "0x1fae896f3041d7e8Bf5Db08cAd6518b0Eb82164a",
  ];

  const merkleTree = new MerkleTree(
    claimerList.map((user) => hashToken(user as `0x${string}`)),
    keccak256,
    { sortPairs: true },
  );
  const merkleTreeRoot = merkleTree.getHexRoot();
  console.log("merkleTree Root:", merkleTreeRoot);

  let message = new Date().getTime().toString();

  let lock = ZERO_BYTES32;
  if (password) {
    lock = await calculatePublicSignals(password);
  }

  let redpacketID = "";

  async function createRedpacket() {
    // create_red_packet
    let creationParams = {
      _merkleroot: merkleTreeRoot,
      _lock: lock,
      _number: claimerList.length,
      _ifrandom: true,
      _duration: 259200,
      _message: message,
      _name: "cache",
      _token_type: 1,
      _token_addr: testTokenAddress,
      _total_tokens: parseEther("1"),
    };

    let createRedPacketTx = await redPacket.create_red_packet(
      ...Object.values(creationParams),
    );
    const createRedPacketRes = await createRedPacketTx.wait();
    const creationEvent = createRedPacketRes.logs.find(
      (_log: EventLog) =>
        typeof _log.fragment !== "undefined" &&
        _log.fragment.name === "CreationSuccess",
    );
    if (creationEvent) {
      const [
        total,
        id,
        name,
        message,
        creator,
        creation_time,
        token_address,
        number,
        ifrandom,
        duration,
        hash_lock,
      ] = creationEvent.args;
      redpacketID = id;

      console.log(
        `\nCreationSuccess Event, total: ${total.toString()}\tRedpacketId: ${id} \n`,
      );
      console.log(`lock: ${hash_lock}`);
    } else {
      throw "Can't parse CreationSuccess Event";
    }

    console.log("Create Red Packet successfully");
  }

  // claim
  async function cliamRedPacket(user) {
    let merkleProof = merkleTree.getHexProof(hashToken(user.address));
    const balanceBefore = await testToken.balanceOf(user.address);

    let claimTx: any;
    if (password) {
      const proofRes = await calcProof(password);
      if (proofRes) {
        const {
          proof: { a, b, c },
          publicSignals,
        } = proofRes;
        claimTx = await redPacket
          .claimPasswordRedpacket(redpacketID, merkleProof, a, b, c)
          .catch((err) => console.error(err));
      }
    } else {
      claimTx = await redPacket.claimOrdinaryRedpacket(
        redpacketID,
        merkleProof,
      );
    }

    const createRedPacketRecipt = await claimTx.wait();
    // console.log("createRedPacketRecipt", createRedPacketRecipt);

    const balanceAfter = await testToken.balanceOf(user.address);
    console.log(
      `\nuser ${user.address} has claimd ${balanceAfter - balanceBefore}\n`,
    );
  }

  await createRedpacket();
  await cliamRedPacket(wallet);
}
