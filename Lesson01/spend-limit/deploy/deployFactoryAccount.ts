import { utils, Wallet, Provider } from "zksync-ethers";
import * as ethers from "ethers";
import * as path from "path";
import * as fs from "fs";
import { HardhatRuntimeEnvironment } from "hardhat/types";
import { Deployer } from "@matterlabs/hardhat-zksync-deploy";

// load env file
import dotenv from "dotenv";
import { deployContract, getWallet } from "./utils";
dotenv.config();

export default async function (hre: HardhatRuntimeEnvironment) {
  const wallet = getWallet();
  const deployer = new Deployer(hre, wallet);
  const factoryArtifact = await deployer.loadArtifact("AAFactory");
  const aaArtifact = await deployer.loadArtifact("Account");

  // Bridge funds if the wallet on zkSync doesn't have enough funds.
  // const depositAmount = ethers.utils.parseEther('0.1');
  // const depositHandle = await deployer.zkWallet.deposit({
  //   to: deployer.zkWallet.address,
  //   token: utils.ETH_ADDRESS,
  //   amount: depositAmount,
  // });
  // await depositHandle.wait();

  const factory = await deployContract(
    "AAFactory",
    [utils.hashBytecode(aaArtifact.bytecode)],
    {
      wallet,
    }
  );
  const factoryAddress = await factory.getAddress();
  console.log(`AA factory address: ${factoryAddress}`);

  const aaFactory = new ethers.Contract(
    factoryAddress,
    factoryArtifact.abi,
    wallet
  );

  const owner = Wallet.createRandom();
  console.log("SC Account owner pk: ", owner.privateKey);

  const salt = ethers.ZeroHash;

  const tx = await aaFactory.deployAccount(salt, owner.address, {gasLimit: 800000});
  await tx.wait();

  const abiCoder = new ethers.AbiCoder();
  const accountAddress = utils.create2Address(
    factoryAddress,
    await aaFactory.aaBytecodeHash(),
    salt,
    abiCoder.encode(["address"], [owner.address])
  );

  console.log(`SC Account deployed on address ${accountAddress}`);

  console.log("Funding smart contract account with some ETH");
  await (
    await wallet.sendTransaction({
      to: accountAddress,
      value: ethers.parseEther("0.02"),
    })
  ).wait();

  const DeploymentsDir = path.join(
    __dirname,
    "./Deployments.json"
  );
  let deploymentsData: any = {};
  if (fs.existsSync(DeploymentsDir)) {
    const data = fs.readFileSync(DeploymentsDir);
    try {
      deploymentsData = JSON.parse(data as never);
    } catch (e) {
      console.error("parse :", e);
      deploymentsData = {};
    }
  }
  deploymentsData = {
    ...deploymentsData,
    DEPLOYED_ACCOUNT_OWNER_PRIVATE_KEY: owner.privateKey,
    DEPLOYED_ACCOUNT_ADDRESS: accountAddress,
    RECEIVER_ACCOUNT: owner.address,
  };
  fs.writeFileSync(
    DeploymentsDir,
    JSON.stringify(deploymentsData, null, 2)
  );

  console.log(`Done!`);
}
