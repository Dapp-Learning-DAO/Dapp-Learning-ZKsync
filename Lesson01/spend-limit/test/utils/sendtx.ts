import {
  Wallet,
  Contract,
  Provider,
  utils,
  EIP712Signer,
  types,
} from "zksync-ethers";
import { ethers } from "ethers";

export async function sendTx(
  provider: Provider,
  account: Contract,
  user: Wallet,
  tx: any
) {
  const accountAddress = await account.getAddress();
  tx = {
    ...tx,
    from: accountAddress,
    chainId: (await provider.getNetwork()).chainId,
    nonce: await provider.getTransactionCount(accountAddress),
    type: 113,
    customData: {
      gasPerPubdata: utils.DEFAULT_GAS_PER_PUBDATA_LIMIT,
    } as types.Eip712Meta,
  };

  tx.gasPrice = await provider.getGasPrice();
  if (tx.gasLimit == undefined) {
    try {
      tx.gasLimit = await provider.estimateGas(tx);
    } catch (error) {
      // console.debug("estimateGas error:", error)
    }
  }

  const signedTxHash = EIP712Signer.getSignedDigest(tx);
  const signature = ethers.concat([
    ethers.Signature.from(user.signingKey.sign(signedTxHash)).serialized,
  ]);

  tx.customData = {
    ...tx.customData,
    customSignature: signature,
  };

  return provider.broadcastTransaction(types.Transaction.from(tx).serialized);
}
