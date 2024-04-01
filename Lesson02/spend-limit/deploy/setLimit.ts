import {
  utils,
  Wallet,
  Provider,
  Contract,
  EIP712Signer,
  types,
} from "zksync-ethers";
import * as ethers from "ethers";
import * as path from "path";
import * as fs from "fs";
import { HardhatRuntimeEnvironment } from "hardhat/types";

// load env file
import dotenv from "dotenv";
import { getProvider } from "./utils";
dotenv.config();

const ETH_ADDRESS =
  process.env.ETH_ADDRESS || "0x000000000000000000000000000000000000800A";

export default async function (hre: HardhatRuntimeEnvironment) {
  // load the values after deploying the FactoryAccount
  const AAcountDeploymentsDir = path.join(
    __dirname,
    "./AAcountDeployments.json"
  );
  let AAcountDeployments: any;
  if (fs.existsSync(AAcountDeploymentsDir)) {
    AAcountDeployments = JSON.parse(fs.readFileSync(AAcountDeploymentsDir) as never);
  } else {
    throw "Must deploy SC account first";
  }
  const {
    DEPLOYED_ACCOUNT_OWNER_PRIVATE_KEY,
    DEPLOYED_ACCOUNT_ADDRESS,
    RECEIVER_ACCOUNT,
  } = AAcountDeployments;

  const provider = getProvider();

  const owner = new Wallet(DEPLOYED_ACCOUNT_OWNER_PRIVATE_KEY, provider);

  const accountArtifact = await hre.artifacts.readArtifact("Account");
  const account = new Contract(DEPLOYED_ACCOUNT_ADDRESS, accountArtifact.abi, owner);

  let setLimitTx = await account.setSpendingLimit.populateTransaction(ETH_ADDRESS, ethers.parseEther("0.0005"));

  setLimitTx = {
    ...setLimitTx,
    from: DEPLOYED_ACCOUNT_ADDRESS,
    chainId: (await provider.getNetwork()).chainId,
    nonce: await provider.getTransactionCount(DEPLOYED_ACCOUNT_ADDRESS),
    type: 113,
    customData: {
      gasPerPubdata: utils.DEFAULT_GAS_PER_PUBDATA_LIMIT,
    } as types.Eip712Meta,
    value: BigInt(0),
  };

  setLimitTx.gasPrice = await provider.getGasPrice();
  setLimitTx.gasLimit = await provider.estimateGas(setLimitTx);

  const signedTxHash = EIP712Signer.getSignedDigest(setLimitTx);

  const signature = ethers.concat([ethers.Signature.from(owner.signingKey.sign(signedTxHash)).serialized]);

  setLimitTx.customData = {
    ...setLimitTx.customData,
    customSignature: signature,
  };

  console.log("Setting limit for account...");
  const sentTx = await provider.broadcastTransaction(types.Transaction.from(setLimitTx).serialized);

  await sentTx.wait();

  const limit = await account.limits(ETH_ADDRESS);
  console.log("Account limit enabled?: ", limit.isEnabled);
  console.log("Account limit: ", limit.limit.toString());
  console.log("Available limit today: ", limit.available.toString());
  console.log("Time to reset limit: ", limit.resetTime.toString());
}
