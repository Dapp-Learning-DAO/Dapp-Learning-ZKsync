# Development Tooling

## Block Explorer

The zkSync Era Block Explorer user interface details comprehensive data about transactions, blocks, batches, wallets, tokens, and smart contracts on the zkSync Era network.

- zkSync Era Block Explorer: <https://explorer.zksync.io/>

## zksync-cli

The zkSync Command Line Interface (CLI) is a powerful tool designed to simplify the development and interaction with zkSync. It provides developers with an easy-to-use set of commands to manage local development environment, interact with contracts, manage tokens, and much more.

### install

Simply use commond: `npm -g install zksync-cli` to install zksync-cli.

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

### Contract interaction

## Foundry with zkSync

## hardhat-zksync-toolbox

- hardhat-zksync-solc
- hardhat-zksync-deploy
- hardhat-zksync-chai-matchers
- hardhat-zksync-verify
