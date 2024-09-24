# Develop a ZK redpacket on ZKsync Era

我们将在 ZKsync Era 网络上部署一个 ZK 红包项目，它将满足以下需求：

- 去中心化的红包创建以及领取体验
- 创建者可以设置白名单，名单之外的地址无法领取奖励
- 支持随机金额
- 使用 ZK snark 技术，支持密码红包功能（在不暴露密码明文的基础上，在链上验证密码是否正确）

> ZK 红包项目已经在 DappLearning 官网上线，您可以在 [dapplearning.org/reward](https://dapplearning.org/reward) 体验完整的红包！

## get-started

### 安装依赖

```sh
# snarkjs
npm install -g snarkjs@latest

# circom
# 参考 circom 官网
# https://docs.circom.io/getting-started/installation

# 项目依赖
yarn install
```

### `snarkjs trust setup` 阶段

- 生成 ptau 文件。**注意：ptau 文件不要公开，如果泄露 ZK 电路会有被攻击的风险，可以在完成 setup 阶段后删除 ptau 文件**.

```sh
# Start a new powers of tau ceremony
npx snarkjs powersoftau new bn128 14 pot14_0001.ptau -v

# Contribute to the ceremony
npx snarkjs powersoftau contribute pot14_0001.ptau pot14_0002.ptau --name="Second contribution" -v -e="some random text"

# Apply a random beacon
# 数字是 32 bytes 的随机数字
npx snarkjs powersoftau beacon pot14_0002.ptau pot14_beacon.ptau 0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f 10 -n="Final Beacon"

# Prepare phase 2
# 这一步耗时较长
npx snarkjs powersoftau prepare phase2 pot14_beacon.ptau pot14_final.ptau -v

#
npx snarkjs powersoftau verify pot14_final.ptau
```

- 编写电路 `circuits/datahash.circom`

```circom
pragma circom 2.0.0;

include "../node_modules/circomlib/circuits/poseidon.circom";

template PoseidonHasher() {
    signal input in;
    signal output out;

    component poseidon = Poseidon(1);
    poseidon.inputs[0] <== in;
    out <== poseidon.out;
}

component main = PoseidonHasher();
```

- 编译电路

  - 这一步会在 `datahash_js` 文件夹下生成三个文件: `datahash.wasm`, `generate_witness.js`, `witness_calculator.js`, 用于后续生成证明
  - 以及 `datahash.r1cs` 和 `dahash.sym` 文件，用于完成 trust setup 的第二阶段

```sh
circom circuits/datahash.circom --r1cs --wasm --sym
```

- trust setup 第二阶段，生成 zkey 文件

```sh
# Groth16 requires a trusted ceremony for each circuit.
npx snarkjs groth16 setup datahash.r1cs pot14_final.ptau circuit_0000.zkey

# Contribute to the phase 2 ceremony
npx snarkjs zkey contribute circuit_0000.zkey circuit_final.zkey --name="1st Contributor Name" -e="some random text" -v

# Verify the latest zkey
npx snarkjs zkey verify datahash.r1cs pot14_final.ptau circuit_final.zkey

# Export the verification key
npx snarkjs zkey export verificationkey circuit_final.zkey verification_key.json
```

- **建议删除所有 .ptau 文件**

### 生成 `verifier.sol`

trust setup 完成后，我们需要利用snarkjs自动生成 `verifier.sol` 文件，辅助红包合约进行链上的zksnark验证。

```sh
npx snarkjs zkey export solidityverifier circuit_final.zkey contracts/redpacket/verifier.sol
```

### 编译合约运行测试

```sh
npx hardhat compile

npx hardhat test --network hardhat
```

### 部署到测试网络

```sh
npx hardhat deploy-zksync --script deploy.ts --network zkSyncSepoliaTestnet

# output
Starting deployment process of "HappyRedPacket"...
Wallet address: 0x67a6Ba1b418911EB67AF4E2DfEF80aCBe2cCE0b6
Estimated deployment cost: 0.0042542753 ETH

"HappyRedPacket" was successfully deployed:
 - Contract address: 0x1FB0E4Bd48a09C3569c570B2Cd70200dd84c096F
 - Contract source: contracts/redpacket/HappyRedPacket.sol:HappyRedPacket
 - Encoded constructor arguments: 0x

Requesting contract verification...
Your verification ID is: 26382
Contract successfully verified on ZKsync block explorer!

Starting deployment process of "Groth16Verifier"...
Wallet address: 0x67a6Ba1b418911EB67AF4E2DfEF80aCBe2cCE0b6
Estimated deployment cost: 0.00083167485 ETH

"Groth16Verifier" was successfully deployed:
 - Contract address: 0x3e6b146aBb56c49D7e87f940112971546e761CeB
 - Contract source: contracts/redpacket/verifier.sol:Groth16Verifier
 - Encoded constructor arguments: 0x

Requesting contract verification...
Your verification ID is: 26383
Contract successfully verified on ZKsync block explorer!
Groth16Verifier address: 0x3e6b146aBb56c49D7e87f940112971546e761CeB
```

### 调用脚本与红包合约交互

交互脚本

```sh
npx hardhat deploy-zksync --script interact.ts --network zkSyncSepoliaTestnet

# output
Running script to interact with contract 0x1FB0E4Bd48a09C3569c570B2Cd70200dd84c096F
Wallet address: 0x67a6Ba1b418911EB67AF4E2DfEF80aCBe2cCE0b6
allowance: 115792089237316195423570985008687907853269984665640564039457.584007913129639935
Redpacket Nonce: 3n
merkleTree Root: 0xa49ea2b86a72603f37d4eee7d52b4b0c85fd23e5605603ad82b7dc10d48fd2c5
CreationSuccess Event, total: 1000000000000000000       RedpacketId: 0x945f7018e48934dc180db1a25b1ef14b2ef2b33eb03f6d5806b7ac084648d8fe  
lock: 0x22abfb84e37f8a8623a6486a1158311ed0a25038730b70d9312df8112c4a7e22
Create Red Packet successfully
createRedPacketRecipt ContractTransactionReceipt {
  provider: Provider { _contractAddresses: {} },
  to: '0x1FB0E4Bd48a09C3569c570B2Cd70200dd84c096F',
  from: '0x67a6Ba1b418911EB67AF4E2DfEF80aCBe2cCE0b6',
  contractAddress: null,
  hash: '0x94a675bda6599a22e93b0125f6149870a63ce5f9be8dcbb909698bfbfc851c38',
  index: 0,
  blockHash: '0xa79b740ebe25d38b8560bf52e7afad5a523b0b41ae40502525155602253c7da8',
  blockNumber: 3819025,
  logsBloom: '0x00000000000400000000004000800000000000000000000040000000000000000000010000000000000000000000000000000000000008000000000000000000000000000000040000000008000000000000200000100000000000000000082000000000000000000000000000000000000000000000002000000010000000000000000000000000000004000000000000000000004000000000000000000010000000000000100000000000000000000000000000000000000000000000000400000002008000000000000000200000000000000000000000000000000000000000000000000000000000000000000040000000001000000000000000000000',
  gasUsed: 8811086n,
  blobGasUsed: undefined,
  cumulativeGasUsed: 0n,
  gasPrice: 25000000n,
  blobGasPrice: undefined,
  type: 113,
  status: 1,
  root: '0xa79b740ebe25d38b8560bf52e7afad5a523b0b41ae40502525155602253c7da8'
}
user 0x67a6Ba1b418911EB67AF4E2DfEF80aCBe2cCE0b6 has claimd 700000000000000000
```

