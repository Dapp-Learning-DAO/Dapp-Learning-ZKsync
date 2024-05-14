import { expect } from "chai";
import { Wallet, Provider, Contract } from "zksync-ethers";
import * as hre from "hardhat";
import { Deployer } from "@matterlabs/hardhat-zksync-deploy";
import * as ethers from "ethers";

import { deployContract, fundAccount } from "./utils";

import dotenv from "dotenv";
import { getWallet } from "../deploy/utils";
dotenv.config();


describe("MyERC20", function () {
  let provider: Provider;
  let wallet: Wallet;
  let deployer: Deployer;
  let userWallet: Wallet;
  let erc20: Contract;

  before(async function () {
    provider = new Provider(hre.network.config.url);
    wallet = getWallet();
    deployer = new Deployer(hre, wallet);

    userWallet = Wallet.createRandom();
    userWallet = new Wallet(userWallet.privateKey, provider);

    erc20 = await deployContract(deployer, "MyERC20", ["TestToken", "TTK", 18]);
  });

  it("should mint tokens to the specified address", async function () {
    const amount = BigInt(100);
    await erc20.connect(wallet);
    const tx = await erc20.mint(userWallet.address, amount);
    await tx.wait();
    const balance = await erc20.balanceOf(userWallet.address);
    expect(balance).to.be.eql(amount);
  });

  it("should have correct decimals", async function () {
    const decimals = await erc20.decimals();
    expect(decimals).to.be.eql(18n);
  });
});
