import { parseUnits, keccak256, encodePacked, toHex, erc20Abi } from "viem";
import { expect } from "chai";
import MerkleTree from "merkletreejs";
import {
  getWallet,
  deployContract,
  LOCAL_RICH_WALLETS,
  getProvider,
} from "../deploy/utils";
import { calcProof, calculatePublicSignals, hashToken } from "../utils/index";
import { Wallet, Contract, Provider } from "zksync-ethers";
import { ZeroAddress } from "ethers";

const ZERO_BYTES32 =
  "0x0000000000000000000000000000000000000000000000000000000000000000";

describe("redpacket", function () {
  let provider: Provider;
  let treeRoot: string;
  let erc20: Contract;
  let owner: Wallet;
  let alice: Wallet;
  let bob: Wallet;
  let merkleTree: MerkleTree;
  let groth16Verifier: Contract;
  let redPacket: Contract;

  beforeEach(async function () {
    provider = getProvider();
    owner = new Wallet(LOCAL_RICH_WALLETS[0].privateKey, provider);
    alice = new Wallet(LOCAL_RICH_WALLETS[1].privateKey, provider);
    bob = new Wallet(LOCAL_RICH_WALLETS[2].privateKey, provider);

    const total_supply = parseUnits("1000000", 18);
    erc20 = await deployContract(
      "SimpleToken",
      ["AAA token", "AAA", 18, total_supply.toString()],
      {
        wallet: owner,
        silent: true,
      }
    );

    merkleTree = new MerkleTree(
      [owner, alice, bob].map((user) => hashToken(user.address)),
      keccak256,
      { sortPairs: true }
    );

    // Root
    treeRoot = merkleTree.getHexRoot().toString();

    groth16Verifier = await deployContract("Groth16Verifier", [], {
      wallet: owner,
      silent: true,
    });
    redPacket = await deployContract("HappyRedPacket", [], {
      wallet: owner,
      silent: true,
    });

    // Init red packet
    let initTx = await redPacket.initialize(groth16Verifier.target);

    await initTx.wait();

    await erc20
      .connect(owner)
      .approve(redPacket.target, parseUnits("10000", 18));
  });

  async function createRedpacket(
    totalAmount,
    ifrandom,
    hashLock = ZERO_BYTES32,
    duration = 1 * 24 * 60 * 60,
    message = "some message",
    token_type = 1, // 0 eth, 1 erc20
  ) {
    const name = "Redpacket Name";
    const total_tokens = parseUnits(totalAmount, 18);
    const ownerAddress = (await owner.getAddress()) as `0x${string}`;
    const tokenAddr = token_type === 1 ? erc20.target : ZeroAddress;
    const redpacketId = keccak256(
      encodePacked(["address", "string"], [ownerAddress, message])
    );
    const blockNumber = await provider.getBlockNumber();
    const block = await provider.getBlock(blockNumber);
    const creation_time = block.timestamp + 1;

    await expect(
      redPacket.connect(owner).create_red_packet(
        treeRoot,
        hashLock,
        3, // number
        ifrandom, // ifrandom
        duration, // 1day
        message, // message
        name,
        token_type,
        tokenAddr,
        total_tokens,
        {
          value: token_type === 1 ? 0n: total_tokens,
        }
      )
    )
      .to.emit(redPacket, "CreationSuccess")
      .withArgs(
        total_tokens,
        redpacketId,
        name,
        message,
        owner.address,
        creation_time,
        tokenAddr,
        3,
        ifrandom,
        duration,
        hashLock
      );

    return {
      redpacketId,
      message,
      total_tokens,
    };
  }

  describe("Normal Redpacket", async () => {
    it("create_red_packet() check require conditions", async () => {
      await expect(
        redPacket.connect(owner).create_red_packet(
          treeRoot,
          ZERO_BYTES32,
          3, // number
          true, // ifrandom
          1 * 24 * 60 * 60, // 1day
          "message", // message
          "Redpacket Name",
          1, // token_type
          erc20.target,
          2 // total_tokens
        )
      ).to.rejectedWith("#tokens > #packets");

      await expect(
        redPacket.connect(owner).create_red_packet(
          treeRoot,
          ZERO_BYTES32,
          0, // number
          true, // ifrandom
          1 * 24 * 60 * 60, // 1day
          "message", // message
          "Redpacket Name",
          1, // token_type
          erc20.target,
          parseUnits("300", 18) // total_tokens
        )
      ).to.rejectedWith("At least 1 recipient");

      await expect(
        redPacket.connect(owner).create_red_packet(
          treeRoot,
          ZERO_BYTES32,
          512, // number
          true, // ifrandom
          1 * 24 * 60 * 60, // 1day
          "message", // message
          "Redpacket Name",
          1, // token_type
          erc20.target,
          parseUnits("300", 18) // total_tokens
        )
      ).to.rejectedWith("At most 511 recipients");

      await expect(
        redPacket.connect(owner).create_red_packet(
          treeRoot,
          ZERO_BYTES32,
          3, // number
          true, // ifrandom
          1 * 24 * 60 * 60, // 1day
          "message", // message
          "Redpacket Name",
          2, // token_type
          erc20.target,
          parseUnits("300", 18) // total_tokens
        )
      ).to.rejectedWith("Unrecognizable token type");

      await expect(
        redPacket.connect(owner).create_red_packet(
          treeRoot,
          ZERO_BYTES32,
          3, // number
          true, // ifrandom
          1 * 24 * 60 * 60, // 1day
          "message", // message
          "Redpacket Name",
          1, // token_type
          erc20.target,
          parseUnits("0.3", 18) // total_tokens
        )
      ).to.rejectedWith("At least 0.1 for each user");

      await expect(
        redPacket.connect(owner).create_red_packet(
          treeRoot,
          ZERO_BYTES32,
          3, // number
          true, // ifrandom
          1 * 24 * 60 * 60, // 1day
          "message", // message
          "Redpacket Name",
          0, // token_type
          ZeroAddress,
          parseUnits("3", 18), // total_tokens
          { value: parseUnits("2", 18)}
        )
      ).to.rejectedWith("No enough ETH");

      const { redpacketId, total_tokens } = await createRedpacket("300", true);
      await expect(
        redPacket.connect(owner).create_red_packet(
          treeRoot,
          ZERO_BYTES32,
          3, // number
          true, // ifrandom
          1 * 24 * 60 * 60, // 1day
          "some message", // message
          "Redpacket Name",
          1, // token_type
          erc20.target,
          parseUnits("300", 18) // total_tokens
        )
      ).to.rejectedWith("Redpacket already exists");
    });

    it("create_red_packet() no password", async () => {
      const { redpacketId, total_tokens } = await createRedpacket("300", true);
      const redpacketData = await redPacket.redpacket_by_id(redpacketId);

      expect(redpacketData.creator).to.equal(owner.address);
      expect(redpacketData.lock).to.equal(ZERO_BYTES32);
      expect(redpacketData.merkleroot).to.equal(treeRoot);
    });

    it("claimOrdinaryRedpacket(): Shuold all member could claim.", async () => {
      const duration = 1 * 24 * 60 * 60;
      const { redpacketId, total_tokens } = await createRedpacket(
        "300",
        false,
        ZERO_BYTES32,
        duration
      );

      const snapshotId = await provider.send("evm_snapshot", []);

      let merkleProof;

      merkleProof = merkleTree.getHexProof(hashToken(owner.address));

      await provider.send("evm_increaseTime", [duration]);
      await expect(
        redPacket
          .connect(owner)
          .claimOrdinaryRedpacket(redpacketId, merkleProof)
      ).to.rejectedWith("Expired");
      await provider.send("evm_revert", [snapshotId]);

      merkleProof = merkleTree.getHexProof(hashToken(alice.address));
      await expect(
        redPacket
          .connect(owner)
          .claimOrdinaryRedpacket(redpacketId, merkleProof)
      ).to.rejectedWith("Verification failed, forbidden");

      for (let user of [owner, alice, bob]) {
        const beforeBalance = await erc20.balanceOf(user.address);
        merkleProof = merkleTree.getHexProof(hashToken(user.address));
        await redPacket
          .connect(user)
          .claimOrdinaryRedpacket(redpacketId, merkleProof);
        const afterBalance = await erc20.balanceOf(user.address);
        expect(afterBalance - beforeBalance).to.equal(total_tokens / 3n);

        await expect(
          redPacket
            .connect(user)
            .claimOrdinaryRedpacket(redpacketId, merkleProof)
        ).to.rejectedWith(user !== bob ? "Already claimed" : "Out of stock");
      }
    });

    it("claimOrdinaryRedpacket(ETH): Should all member could claim.", async () => {
      const duration = 1 * 24 * 60 * 60;
      const { redpacketId, total_tokens } = await createRedpacket(
        "300",
        false,
        ZERO_BYTES32,
        duration,
        `some message`,
        0,
      );

      const snapshotId = await provider.send("evm_snapshot", []);

      let merkleProof;

      merkleProof = merkleTree.getHexProof(hashToken(owner.address));

      await provider.send("evm_increaseTime", [duration]);
      await expect(
        redPacket
          .connect(owner)
          .claimOrdinaryRedpacket(redpacketId, merkleProof)
      ).to.rejectedWith("Expired");
      await provider.send("evm_revert", [snapshotId]);

      merkleProof = merkleTree.getHexProof(hashToken(alice.address));
      await expect(
        redPacket
          .connect(owner)
          .claimOrdinaryRedpacket(redpacketId, merkleProof)
      ).to.rejectedWith("Verification failed, forbidden");

      for (let user of [owner, alice, bob]) {
        const beforeBalance = await provider.getBalance(user.address);
        merkleProof = merkleTree.getHexProof(hashToken(user.address));
        const { hash: txHash } = await redPacket
          .connect(user)
          .claimOrdinaryRedpacket(redpacketId, merkleProof);
        const txReceipt = await provider.getTransactionReceipt(txHash);
        let gasFee = 0n;
        if (txReceipt) {
          gasFee = txReceipt.gasUsed * txReceipt.gasPrice;
        }
        const afterBalance = await provider.getBalance(user.address);
        expect(afterBalance - beforeBalance + gasFee).to.equal(total_tokens / 3n);

        await expect(
          redPacket
            .connect(user)
            .claimOrdinaryRedpacket(redpacketId, merkleProof)
        ).to.rejectedWith(user !== bob ? "Already claimed" : "Out of stock");
      }
    });

    it("refund(): Should refund after expired", async () => {
      const duration = 1 * 60 * 60;
      const { redpacketId, total_tokens } = await createRedpacket(
        "300",
        false,
        ZERO_BYTES32,
        duration
      );

      const snapshotId = await provider.send("evm_snapshot", []);
      await expect(
        redPacket.connect(alice).refund(redpacketId)
      ).to.rejectedWith("Creator Only");

      await expect(
        redPacket.connect(owner).refund(redpacketId)
      ).to.rejectedWith("Not expired yet");

      // claim all tokens
      for (let user of [owner, alice, bob]) {
        const merkleProof = merkleTree.getHexProof(hashToken(user.address));
        await redPacket
          .connect(user)
          .claimOrdinaryRedpacket(redpacketId, merkleProof);
      }

      await provider.send("evm_increaseTime", [duration]);

      await expect(
        redPacket.connect(owner).refund(redpacketId)
      ).to.rejectedWith("None left in the red packet");

      // revert to snapshot
      await provider.send("evm_revert", [snapshotId]);
      await provider.send("evm_increaseTime", [duration]);

      const beforeBalance = await erc20.balanceOf(owner.address);

      await expect(redPacket.connect(owner).refund(redpacketId))
        .to.emit(redPacket, "RefundSuccess")
        .withArgs(redpacketId, erc20.target, total_tokens, ZERO_BYTES32);

      const afterBalance = await erc20.balanceOf(owner.address);
      expect(afterBalance - beforeBalance).to.equal(total_tokens);
    });
  });

  describe("ZK Redpacket", async () => {
    it("create_red_packet() with password", async () => {
      const password = "This is a password";
      const hashLock = await calculatePublicSignals(password);
      const { redpacketId, total_tokens } = await createRedpacket(
        "300",
        true,
        hashLock
      );
      const redpacketData = await redPacket.redpacket_by_id(redpacketId);

      expect(redpacketData.creator).to.equal(owner.address);
      expect(redpacketData.lock).to.equal(hashLock);
      expect(redpacketData.merkleroot).to.equal(treeRoot);
    });

    it("claimPasswordRedpacket(): Shuold all member could claim with correct password.", async () => {
      const correct_password = "This is a correct password";
      const hashLock = await calculatePublicSignals(correct_password);
      const { redpacketId, total_tokens } = await createRedpacket(
        "300",
        false,
        hashLock
      );
      const { redpacketId: redpacketId2 } = await createRedpacket(
        "300",
        false,
        ZERO_BYTES32,
        1 * 24 * 60 * 60,
        "some other message"
      );

      let merkleProof;

      const wrong_password = "This is a wrong password";
      const { proof: wrong_zkproof, publicSignals: wrong_publicSignals } =
        (await calcProof(wrong_password)) as never;

      expect(toHex(BigInt(wrong_publicSignals[0]))).to.not.equal(hashLock);

      merkleProof = merkleTree.getHexProof(hashToken(owner.address));
      await expect(
        redPacket
          .connect(owner)
          .claimPasswordRedpacket(
            redpacketId2,
            merkleProof,
            wrong_zkproof.a,
            wrong_zkproof.b,
            wrong_zkproof.c
          )
      ).to.rejectedWith("Not password redpacket");

      merkleProof = merkleTree.getHexProof(hashToken(owner.address));
      await expect(
        redPacket
          .connect(owner)
          .claimPasswordRedpacket(
            redpacketId,
            merkleProof,
            wrong_zkproof.a,
            wrong_zkproof.b,
            wrong_zkproof.c
          )
      ).to.rejectedWith("ZK Verification failed, wrong password");

      const { proof: correct_zkproof, publicSignals } = (await calcProof(
        correct_password
      )) as never;

      expect(toHex(BigInt(publicSignals[0]))).to.equal(hashLock);

      const claimed_value = total_tokens / 3n;

      for (let user of [owner, alice, bob]) {
        const beforeBalance = await erc20.balanceOf(user.address);
        merkleProof = merkleTree.getHexProof(hashToken(user.address));
        await expect(
          redPacket
            .connect(user)
            .claimPasswordRedpacket(
              redpacketId,
              merkleProof,
              correct_zkproof.a,
              correct_zkproof.b,
              correct_zkproof.c
            )
        )
          .to.emit(redPacket, "ClaimSuccess")
          .withArgs(
            redpacketId,
            user.address,
            claimed_value,
            erc20.target,
            hashLock
          );

        const afterBalance = await erc20.balanceOf(user.address);
        expect(afterBalance - beforeBalance).to.equal(claimed_value);
      }
    });

    it("refund(): Should refund after expired", async () => {
      const duration = 1 * 60 * 60;
      const password = "This is a password";
      const hashLock = await calculatePublicSignals(password);
      const { redpacketId, total_tokens } = await createRedpacket(
        "300",
        false,
        hashLock,
        duration
      );

      await provider.send("evm_increaseTime", [duration]);

      const beforeBalance = await erc20.balanceOf(owner.address);

      await expect(redPacket.connect(owner).refund(redpacketId))
        .to.emit(redPacket, "RefundSuccess")
        .withArgs(redpacketId, erc20.target, total_tokens, hashLock);

      const afterBalance = await erc20.balanceOf(owner.address);
      expect(afterBalance - beforeBalance).to.equal(total_tokens);
    });
  });
});
