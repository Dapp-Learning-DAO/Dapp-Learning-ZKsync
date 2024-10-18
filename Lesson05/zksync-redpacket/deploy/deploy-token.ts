import { parseUnits } from "ethers";
import { deployContract, saveDeployment } from "./utils";
const { ethers } = require("hardhat");

// An example of a basic deploy script
// It will deploy a Greeter contract to selected network
// as well as verify it on Block Explorer if possible for the network
export default async function () {
  const simpleToken = await deployContract("SimpleToken", [
    "DappLearning Test Token",
    "DLD",
    18,
    parseUnits("10000000", 18).toString(),
  ]);
  const simpleTokenAddress = await simpleToken.getAddress();

  console.log("SimpleToken address:", simpleTokenAddress);

  saveDeployment({
    SimpleToken: simpleTokenAddress,
  });
}
