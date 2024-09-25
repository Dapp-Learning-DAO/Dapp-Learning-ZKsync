import { deployContract, saveDeployment } from "./utils";

export default async function () {
  const redPacket = await deployContract("HappyRedPacket");
  const redPacketAddress = await redPacket.getAddress();

  const groth16Verifier = await deployContract("Groth16Verifier");
  const groth16VerifierAddress = await groth16Verifier.getAddress();

  console.log("Groth16Verifier address:", groth16VerifierAddress);

  let initRecipt = await redPacket.initialize(groth16VerifierAddress);

  await initRecipt.wait();

  saveDeployment({
    Redpacket: redPacketAddress,
    Verifier: groth16VerifierAddress,
  });
}
