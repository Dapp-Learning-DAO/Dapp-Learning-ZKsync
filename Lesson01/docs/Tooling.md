# Development Tooling

## Block Explorer

The zkSync Era Block Explorer user interface details comprehensive data about transactions, blocks, batches, wallets, tokens, and smart contracts on the zkSync Era network.

- zkSync Era Block Explorer: <https://explorer.zksync.io/>

## zksync-cli

The zkSync Command Line Interface (CLI) is a powerful tool designed to simplify the development and interaction with zkSync. It provides developers with an easy-to-use set of commands to manage local development environment, interact with contracts, manage tokens, and much more.

zksync 开发脚手架，可以让大家方便的使用模板创建项目，查询链上的交易数据，合约状态，账户状态，与链上合约交互，与官方跨链桥交互等等。

### install

Simply use commond (recommand): `npx zksync-cli`

```sh
npx zksync-cli [options] [command]
```

Or `npm -g install zksync-cli` to install zksync-cli.

```sh
npm -g install zksync-cli
```

### Commonds

- `npx zksync-cli dev`

  - `npx zksync-cli dev start` initiates your local environment.
  - `npx zksync-cli dev stop` terminates the local environment.
  - `npx zksync-cli dev restart [module name]` restarts your environment or specific modules.

- `npx zksync-cli create` simplifies the initial project setup by providing templates in three main categories:
  - Frontend
    - Vue Template
    - React Template
    - Svelte Template
  - Contracts
    - Ethers v6 (latest)
      - Solidity Template
      - Vyper Template
    - Ethers v5
      - Solidity Template
      - Vyper Template
  - Script
    - Viem Template
    - Ethers v6 Template
    - Ethers v5 Template

## zksync-ethers

- [zksync-ethers](https://www.npmjs.com/package/zksync-ethers)
  - v6 对应 ehters v6 (推荐)
  - v5 对应 ehters v5

## hardhat-zksync

- [@matterlabs/hardhat-zksync](https://www.npmjs.com/package/@matterlabs/hardhat-zksync)
  - @matterlabs/hardhat-zksync-node
  - @matterlabs/hardhat-zksync-solc
  - @matterlabs/hardhat-zksync-deploy
  - @matterlabs/hardhat-zksync-verify
  - @matterlabs/hardhat-zksync-upgradable
  - @matterlabs/hardhat-zksync-ethers

## Foundry with zkSync

TODO
