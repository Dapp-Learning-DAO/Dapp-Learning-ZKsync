# Dapp-Learning ZKsync Era Tutorial

<div>
  <p>
    <a href="https://github.com/Dapp-Learning-DAO/Dapp-Learning/tree/main/docs/imgs/wechat-group-helper.png"><img alt="Wechat group helper" src="https://img.shields.io/static/v1?&label=&logo=wechat&message=wechat group&color=brightgreen&logoColor=white"></a>
    <a href="https://twitter.com/Dapp_Learning"><img alt="Twitter Follow" src="https://img.shields.io/twitter/follow/dapp_learning?label=Follow"></a>
    <a href="https://www.youtube.com/c/DappLearning"><img alt="YouTube Channel Subscribers" src="https://img.shields.io/youtube/channel/subscribers/UCdJKZVxO55N3n2BQYXMDAcQ?style=social"></a>
    <a href="https://discord.gg/cRYNYXqPeR"><img src="https://img.shields.io/discord/907080577096757279?color=5865F2&logo=discord&logoColor=white&label=discord" alt="Discord server" /></a>
    <a href="https://t.me/joinchat/48Mp2jy4Yw40MmI1"><img src="https://img.shields.io/badge/telegram-blue?color=blue&logo=telegram&logoColor=white" alt="Telegram group" /></a>
  </p>
</div>

## 课程概览

- **课程形式**: 视频直播课程 + 代码示例 + 线上workshop
- **课程时长**: 6 周
- **课程目标:** 通过学习本课程，学习者将理解 zkSycn Era 的工作原理，并有能力在 ZKsync Era 网络上部署 DApp。 帮助更多用户使用 ZKsync Era。

## **课程大纲**

### Lesson 1: 快速上手 ZKsync 开发以及 Native AA

- **开发工具**: 介绍开发工具的用法 (Block Explorer, ZKsync-cli, hardhat-plugins).
- **入门示例**: 在 ZKsync Era 网络上部署 ERC20, ERC721 合约并交互.
- **Native AA**: 在 ZKsync Era 网络上部署 Native AA 示例 (spend-limit)，并交互.

[video p1](https://www.youtube.com/watch?v=vWIEDMvpqFE&list=PLgPVMJY4tnFNK260S6thZqEAXJhtcgHaW&index=4) | [video p2](https://www.youtube.com/watch?v=b9cToAol3cg&list=PLgPVMJY4tnFNK260S6thZqEAXJhtcgHaW&index=3) | [Doc](./Lesson01/README.md)

### Lesson 2: ZKsync Era 抽象账户

- **EIP-4337 概述**: 介绍 EIP-4337 及其工作原理。
- **ZKsync Era 中的 AA 机制**: 介绍 ZKsync Era 中的账户抽象（AA）机制，详细说明其操作原理和工作流程.
- **原生 AA 与 EIP-4337 的比较**: 比较 ZKsync Era 中原生 AA 与 EIP-4337 的差异.
- **费用模型与Paymaster**: 介绍 ZKsync Era 中账户抽象的费用模型以及Paymaster.

[video](https://www.youtube.com/watch?v=gGTnBRnSFh8&list=PLgPVMJY4tnFNK260S6thZqEAXJhtcgHaW&index=2) | [Doc](./Lesson02/README.md)

### Lesson 3: ZKsync 系统特性详解

- **L1-L2 Message**: 链接 主网 和 ZKsync Era 的合约，用于证明验证以及 L2 <-> L1 通信.
- **System Contracts**: ZKsync Era 系统合约介绍.
- **Gas Fee Model**: 解析 ZKsync Era 的 gas 费用机制及其与以太坊的区别.
- **Differences from Ethereum**: 详解 ZKsync Era 与主网的区别以及开发注意事项.
- **ZKsync Bridge**: 介绍 ZKsync 桥的工作方式.

[video](https://www.youtube.com/watch?v=yUMCUgTVj5U&list=PLgPVMJY4tnFNK260S6thZqEAXJhtcgHaW&index=4) | [Doc](./Lesson03/README.md)

### Lesson 4: DApp 部署 1

- 如何在 ZKsync Era 上部署 Uniswap V3 (front-end + contracts).

[video](https://www.youtube.com/watch?v=XuBzfrhGReM&list=PLgPVMJY4tnFNK260S6thZqEAXJhtcgHaW&index=5) | [Doc](./Lesson04/README.md)

### Lesson 5: DApp 部署 2

- **ZK red packet**: 如何在 ZKsync Era 上部署 ZK Redpacket (front-end + contracts).

### Lesson 6: DApp 部署 3

- **rollup bridge**: 如何在 ZKsync Era 上部署 Rollup Bridge.

</br>

**Rollups 原理和 ZKsync Era (选修)**:

### Lesson 7: Boojum 原理 1

- **Boojum**: Boojum 原理 电路算术化
  - [video](https://www.youtube.com/watch?v=MrOLmEmlBfM&list=PLgPVMJY4tnFNK260S6thZqEAXJhtcgHaW&index=1) | [Doc](./boojum-01/README.md)
- **Plonk and Plonky2**: Plonk和Plonky2技术的技术原理。

### Lesson 8: Boojum 原理 2

- **Lookup Argument**: 查找表技术

[video p1](https://www.youtube.com/watch?v=1Jzk1zQA6H4&list=PLgPVMJY4tnFNK260S6thZqEAXJhtcgHaW&index=7) | [video p2](https://www.youtube.com/watch?v=XLsbKFysSt4&list=PLgPVMJY4tnFNK260S6thZqEAXJhtcgHaW&index=8) | [Doc](./boojum-02/README.md)

### Lesson 9: Boojum 原理 3

- **Boojum 深入**: FFT

[video p1](https://www.youtube.com/watch?v=JAfwbs_Ymnk&list=PLgPVMJY4tnFNK260S6thZqEAXJhtcgHaW&index=9) | [video p2](https://www.youtube.com/watch?v=BDLOuGmb7mk&list=PLgPVMJY4tnFNK260S6thZqEAXJhtcgHaW&index=10) | [Doc](./boojum-02/README.md)

## **Course Overview**

- **Course Format**: online live video courses + code example + online workshops
- **Duration**: 6 weeks
- **Course Objectives:** Upon completing the course, learners will understand how ZKsync Era works and be able to independently develop DApps for ZKsync Era. Help more users onboard ZKsync Era.

## **Course Outline**

### Lesson 1: Quick Introduction to ZKsync Development and Native AA

- **Development tools**: Introduction and usage of development tools (Block Explorer, ZKsync-cli, hardhat-plugins, Foundry).
- **Simple Example**: Deploying and interacting with ERC20, ERC721 contracts on the ZKsync Era network.
- **Native AA**: Deploying and interacting with a simple AA contract (spend-limit).

[video p1](https://www.youtube.com/watch?v=vWIEDMvpqFE&list=PLgPVMJY4tnFNK260S6thZqEAXJhtcgHaW&index=4) | [video p2](https://www.youtube.com/watch?v=b9cToAol3cg&list=PLgPVMJY4tnFNK260S6thZqEAXJhtcgHaW&index=3) | [Doc](./Lesson01/README.md)

### Lesson 2: ZKsync Era Account Abstraction

- **EIP 4337 overview**: An overview of EIP 4337 and how it works.
- **AA Mechanism in ZKsync Era**: Introduce the Account Abstraction (AA) mechanism in ZKsync Era, detailing its operating principles and workflow.
- **Native AA vs EIP 4337**: Compare the differences between native AA in ZKsync Era,
- **Fee Model and Paymaster**: Introduce the Fee Model for Account Abstraction in ZKsync Era and the role of Paymasters.

[video](https://www.youtube.com/watch?v=gGTnBRnSFh8&list=PLgPVMJY4tnFNK260S6thZqEAXJhtcgHaW&index=2) | [Doc](./Lesson02/README.md)

### Lesson 3: ZKsync System Features

- **L1-L2 Message**: contracts connecting Ethereum and ZKsync Era for proof validation and L2 <-> L1 communications.
- **System Contracts**: Introduce the ZKsync Era system contracts.
- **Gas Fee Model**: Explain the ZKsync Era gas fee mechanism and its difference from Ethereum.
- **Differences from Ethereum**: Explain the differences from ethereum and development consideration.

[video](https://www.youtube.com/watch?v=yUMCUgTVj5U&list=PLgPVMJY4tnFNK260S6thZqEAXJhtcgHaW&index=4) | [Doc](./Lesson03/README.md)

### Lesson 4: DApp Development 1

- How to deploy Uniswap V3 on ZKsync Era (front-end + contracts)

[video](https://www.youtube.com/watch?v=XuBzfrhGReM&list=PLgPVMJY4tnFNK260S6thZqEAXJhtcgHaW&index=5) | [Doc](./Lesson04/README.md)

### Lesson 5: DApp Development 2

- **ZK red packet**: Develop a ZK red packet product (contract + frontend) on ZKsync Era.

### Lesson 6: DApp Development 3

- **rollup bridge**: How to develop a cross rollup bridge on ZKsync Era.

</br>

**Principles of Rollups and ZKsync Era (Optional)**:

### Lesson 7: Principles of Boojum 1

- **Boojum**: Technical principles of Boojum, Circuit Arithmetization

[video](https://www.youtube.com/watch?v=MrOLmEmlBfM&list=PLgPVMJY4tnFNK260S6thZqEAXJhtcgHaW&index=1) | [Doc](./boojum-01/README.md)

### Lesson 8: Principles of Boojum 2

- **Lookup Argument**:

[video p1](https://www.youtube.com/watch?v=1Jzk1zQA6H4&list=PLgPVMJY4tnFNK260S6thZqEAXJhtcgHaW&index=7) | [video p2](https://www.youtube.com/watch?v=XLsbKFysSt4&list=PLgPVMJY4tnFNK260S6thZqEAXJhtcgHaW&index=8) | [Doc](./boojum-02/README.md)

### Lesson 9: Principles of Boojum 3

- **Inside Boojum**: FFT

[video p1](https://www.youtube.com/watch?v=JAfwbs_Ymnk&list=PLgPVMJY4tnFNK260S6thZqEAXJhtcgHaW&index=9) | [video p2](https://www.youtube.com/watch?v=BDLOuGmb7mk&list=PLgPVMJY4tnFNK260S6thZqEAXJhtcgHaW&index=10) | [Doc](./boojum-02/README.md)

## Reference

- [ZKsync Era Doc](https://docs.ZKsync.io/)
