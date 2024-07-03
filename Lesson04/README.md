# Lesson 4: DApp Development 1

> How to deploy Uniswap V3 on ZKsync Era (front-end + contracts)

本次课程我们将尝试在 ZKsync Era Sepolia Testnet 上部署一套 UniswapV3 合约，以及搭建一个适配的前端项目。

> 本文档出于教学目的，所使用的代码为较老版本，请勿将其用于生产 (Uniswap 官方已在 6 月中旬支持了 ZKsync Era 网络)

## era-uniswap-deploy-v3

[era-uniswap-deploy-v3](https://github.com/uniswap-zksync/era-uniswap-deploy-v3) 是 [uniswap deploy-v3](https://github.com/Uniswap/deploy-v3) 的 fork 仓库，主要针对 ZKsync Era 网络进行了适配修改。

由于 Uniswap V3 的合约分为好几个部分，例如 v3-core, v3-periphery, 且彼此之间有着依赖关系，所以为了便于统一部署以及管理，Uniswap 专门创建了 `deploy-v3` 仓库，实现了脚本一键自动部署。

### deploy flow

主要实现思路是将整个部署流程编写为 hardhat task 的形式 (`deploy-v3`)，将每个部署的步骤拆分为单独的文件，实现灵活的操作配置。脚本没有使用 hardhat 的部署插件，而是自己实现的逻辑。

![uniswap-deploy-v3.drawio](./docs/img/uniswap-deploy-v3.drawio.svg)

1. 部署 `UniswapV3Factory` 合约，传入 `WETH` 合约地址
2. 增加 0.01% 费率档位（默认只有 1%, 0.3%, 0.05% 的费率档位）
3. 部署 `UniswapInterfaceMulticall` 合约
4. 部署 `ProxyAdmin` 合约 (作为 NFT Manager owner)
5. 部署 `TickLens` 合约 (用于查询 pool 的 tick 信息)
6. 部署 `NFTDescriptor` 合约 (library)
7. 部署 `NonfungibleTokenPositionDescriptor` 合约 (用于保存 LP NFT 信息)
8. 部署 `TransparentUpgradeableProxy` 合约 (透明代理合约)，将 7 逻辑合约，4 owner 作为初始化参数
9. 部署 `NonfungiblePositionManager` 合约，将 8 `_tokenDescriptor`, 1 `_factory` 以及 WETH 地址 作为初始化参数
10. 部署 `V3Migrator` 合约，将 1 `_factory` 以及 WETH 地址 作为初始化参数
11. 设置 `UniswapV3Factory` 的 owner 为 deployer
12. 部署 `UniswapV3Staker`, 将 1 `_factory` 以及 WETH 地址 作为初始化参数
13. 部署 `QuoterV2`, 将 1 `_factory` 以及 WETH 地址 作为初始化参数
14. 部署 `SwapRouter`, 将 1 `_factory` 以及 WETH 地址 作为初始化参数

era-uniswap-deploy-v3 在原仓库的基础之上，加入了 ZKsync Era 的相关适配：

- 安装 `hardhat-zksync` 相关插件
- `hardhat.config.ts` 中增加 ZKsync Era 网络配置，和 zksolc 配置

  ```ts
  networks: {
    ZKsyncSepoliaTestnet: {
      url: "https://sepolia.era.zksync.dev", // The testnet RPC URL of ZKsync Era network.
      ethNetwork: "sepolia", // The Ethereum Web3 RPC URL, or the identifier of the network (e.g. `mainnet` or `sepolia`)
      zksync: true,
      // Verification endpoint for Sepolia
      verifyURL: 'https://explorer.sepolia.era.zksync.dev/contract_verification'
    },
    hardhat: {
      zksync: true,
    },
  },
  zksolc: {
    version: '1.3.13',
    compilerSource: 'binary',
    settings: {
      metadata: {
        bytecodeHash: 'none',
      },
    },
  },
  ```

- 修改依赖安装路径，改为相应的 zksync fork 版本;
  - 例如 `@uniswap/v3-core` node_modules 依赖，需要指向 fork 版本，其中主要是提供了 `zksolc` 编译的适配 zksync 的合约编译文件
  - `@uniswap/swap-router-contracts`
  - `@uniswap/v3-core`
  - `@uniswap/v3-periphery`
  - `@uniswap/v3-staker`
  - `era-openzeppelin-contracts`
  - `v3-periphery-1_3_0`
- 修改最低 node 版本为 16 (原版 14)
- 当某个合约依赖于另一个独立部署的 `library` 合约，需要特殊处理，在 zksolc 配置中添加相应的 libraries 配置 (zksync 编译机制的特殊性)

  ```ts
  // src/steps/deploy-nft-position-descriptor-v1_3_0.ts
  async computeArtifact(state) {
    ...
    //
    hre.config.zksolc.settings.libraries = {
      'v3-periphery-1_3_0/contracts/libraries/NFTDescriptor.sol': {
        NFTDescriptor: state.nftDescriptorLibraryAddressV1_3_0,
      },
    }
    await hre.run('compile')
    ...
  },
  ```

### run deploy

运行下列命令，替换相关变量数值，脚本将自动部署一整套合约到 ZKsync Era Sepolia Testnet 上，并将部署合约的地址存入 `state.json` 文件。

> 在 ZKsync Era sepolia 网络中没有官方提供的 WETH 合约，所以我们需要自己部署一个 WETH9，或者你也可以直接使用我部署的 WETH9 合约，地址是 `0x528499043839E2021Acce95fdf7C438692dc3c04`

```sh
yarn start --json-rpc https://sepolia.era.zksync.dev --native-currency-label ETH --owner-address ${WALLET_ADDRESS} --private-key ${WALLET_PRIVATE_KEY} --weth9-address ${WETH9_ADDRESS}
```

命令行输出

```sh
Step 1 complete [
  {
    message: 'Contract UniswapV3Factory deployed',
    address: '0x9C903Bed41c2E3e6CA60EDbC1f0EEBA1CAF4926A',
    constructorArgs: [],
    hash: '0xca23bd56580b7dcfab05be035f8e325173494c26473727c6b7dbcacfecab8215'
  }
]
Step 2 complete [
  {
    message: 'UniswapV3Factory added a new fee tier 1 bps with tick spacing 1',
    hash: '0x279bd5ab3848fa966a156c76c01d568482c48de4244ae9db5276f9449aa2e2f8'
  }
]
Step 3 complete [
  {
    message: 'Contract UniswapInterfaceMulticall deployed',
    address: '0xa872e6dee1F865734DfaCB754f9C6Ca06644B5B6',
    constructorArgs: [],
    hash: '0x1227e433705a36b39c0372dc4831d9ea8badcee6203657134b0d9e6074fece7c'
  }
]
Step 4 complete [
  {
    message: 'Contract ProxyAdmin deployed',
    address: '0x076EA0dF4964efE0D89915E9C635086691575722',
    constructorArgs: [],
    hash: '0xe0ee632bf383feefa61c980cc37f74442bd9ca033737e54ee2f57dcf12874f98'
  }
]
Step 5 complete [
  {
    message: 'Contract TickLens deployed',
    address: '0xF4a8d30e73253C9E9D8BF1325db650CCd7EB48d7',
    constructorArgs: [],
    hash: '0xb8f727173cd44d1d42631cc43650ed4ad86e1bc35c6333f4a2b90580c821569a'
  }
]
Step 6 complete [
  {
    message: 'Library NFTDescriptor deployed',
    address: '0x3a78cCBf86496E7AC1330362B81c371096D42dBF',
    hash: '0xfc2257d9dfb36f4a5e4344319eaf41cb5459de7a8d173c9581a4098180ef2d6e'
  }
]
Compiling 36 Solidity files
base64-sol/base64.sol: Warning: Source file does not specify required compiler version! Consider adding "pragma solidity ^0.7.6;"

Successfully compiled 36 Solidity files
Step 7 complete [
  {
    message: 'Contract NonfungibleTokenPositionDescriptor deployed',
    address: '0x91D970f2773D1f855FaDF1Df5104cB0C9CdfD9c8',
    constructorArgs: [
      '0x528499043839E2021Acce95fdf7C438692dc3c04',
      '0x4554480000000000000000000000000000000000000000000000000000000000'
    ],
    hash: '0x005ee48a2643dc8dc99e6ac7e1be9c9f9cd2d36c9bd38642ae810090eed51296'
  }
]
Step 8 complete [
  {
    message: 'Contract TransparentUpgradeableProxy deployed',
    address: '0x13E3c8154Aa2fD06CE742D0B32f94f233390FdAa',
    constructorArgs: [
      '0x91D970f2773D1f855FaDF1Df5104cB0C9CdfD9c8',
      '0x076EA0dF4964efE0D89915E9C635086691575722',
      '0x'
    ],
    hash: '0x32af718885a6bb28da4f5d026afa20018da3103e99a80e9208dc08e0d0d9d9e8'
  }
]
Step 9 complete [
  {
    message: 'Contract NonfungiblePositionManager deployed',
    address: '0xa8514d8f80056b5A024F8193089aBDfe818b66fA',
    constructorArgs: [
      '0x9C903Bed41c2E3e6CA60EDbC1f0EEBA1CAF4926A',
      '0x528499043839E2021Acce95fdf7C438692dc3c04',
      '0x13E3c8154Aa2fD06CE742D0B32f94f233390FdAa'
    ],
    hash: '0x6234fd2868d031091dba2592b1aef51afef89cfb487d36cec333c3a232f532f4'
  }
]
Step 10 complete [
  {
    message: 'Contract V3Migrator deployed',
    address: '0xf72A93796F1094b1faDFB4658b1c489fD27da522',
    constructorArgs: [
      '0x9C903Bed41c2E3e6CA60EDbC1f0EEBA1CAF4926A',
      '0x528499043839E2021Acce95fdf7C438692dc3c04',
      '0xa8514d8f80056b5A024F8193089aBDfe818b66fA'
    ],
    hash: '0x05aa6e6d011901ffc8adff194055e517d58c14df213897fa1c2596031eb59c29'
  }
]
Step 11 complete [
  {
    message: 'UniswapV3Factory owned by 0xe45d43FEb3F65B4587510A68722450b629154e6f already'
  }
]
Step 12 complete [
  {
    message: 'Contract UniswapV3Staker deployed',
    address: '0xbe6B00Fb0DabFEf7F2D31af93310D30f9d0357D5',
    constructorArgs: [
      '0x9C903Bed41c2E3e6CA60EDbC1f0EEBA1CAF4926A',
      '0xa8514d8f80056b5A024F8193089aBDfe818b66fA',
      2592000,
      63072000
    ],
    hash: '0x820ea99e97c0c8f1b4256eddbb8e15cffec4717d643b81523d2c067990b15854'
  }
]
Step 13 complete [
  {
    message: 'Contract QuoterV2 deployed',
    address: '0x2371D895e055B85b78cbf500959D02Db7c19362A',
    constructorArgs: [
      '0x9C903Bed41c2E3e6CA60EDbC1f0EEBA1CAF4926A',
      '0x528499043839E2021Acce95fdf7C438692dc3c04'
    ],
    hash: '0x567cfd1cc574c243af35a6392e9baf7944526f60fc01b2c900c26a9742142031'
  }
]
Step 14 complete [
  {
    message: 'Contract SwapRouter02 deployed',
    address: '0x47514ab4b7aFbBcbd83F69C130d861375BE56538',
    constructorArgs: [
      '0x0000000000000000000000000000000000000000',
      '0x9C903Bed41c2E3e6CA60EDbC1f0EEBA1CAF4926A',
      '0xa8514d8f80056b5A024F8193089aBDfe818b66fA',
      '0x528499043839E2021Acce95fdf7C438692dc3c04'
    ],
    hash: '0x4d2bf689d8dbf06331300a55752e03d1c67185c0195e884a62a40ce2c24b4225'
  }
]
Step 15 complete [
  {
    message: 'ProxyAdmin owned by 0xe45d43FEb3F65B4587510A68722450b629154e6f already'
  }
]
Deployment succeeded
...
Final state
{"v3CoreFactoryAddress":"0x9C903Bed41c2E3e6CA60EDbC1f0EEBA1CAF4926A","multicall2Address":"0xa872e6dee1F865734DfaCB754f9C6Ca06644B5B6","proxyAdminAddress":"0x076EA0dF4964efE0D89915E9C635086691575722","tickLensAddress":"0xF4a8d30e73253C9E9D8BF1325db650CCd7EB48d7","nftDescriptorLibraryAddressV1_3_0":"0x3a78cCBf86496E7AC1330362B81c371096D42dBF","nonfungibleTokenPositionDescriptorAddressV1_3_0":"0x91D970f2773D1f855FaDF1Df5104cB0C9CdfD9c8","descriptorProxyAddress":"0x13E3c8154Aa2fD06CE742D0B32f94f233390FdAa","nonfungibleTokenPositionManagerAddress":"0xa8514d8f80056b5A024F8193089aBDfe818b66fA","v3MigratorAddress":"0xf72A93796F1094b1faDFB4658b1c489fD27da522","v3StakerAddress":"0xbe6B00Fb0DabFEf7F2D31af93310D30f9d0357D5","quoterV2Address":"0x2371D895e055B85b78cbf500959D02Db7c19362A","swapRouter02":"0x47514ab4b7aFbBcbd83F69C130d861375BE56538"}
✨  Done in 358.65s.
```

等待部署完成，我们能在命令行输出以及 `state.json` 文件中找到合约地址。

## uniswap-zksync-interface

在成功部署了 UniswapV3 合约之后，我们现在来实现前端项目。你可以用 Uniswap 官方的前端项目 [interface](https://github.com/Uniswap/interface) fork 一个，或者直接使用我修改后的版本 [uniswap-zksync-interface](https://github.com/0x-stan/uniswap-zksync-interface).

由于最新版本的 interface 代码包含了 Uniswap Wallet，代码量更加庞大，所以我们将从 [v4.160.0](https://github.com/Uniswap/interface/tree/v4.160.0) 版本修改，以便于我们更清晰的学习源码。

主要在以下方面修改：

- 安装 `zksync-ethers`，修改最低 node 版本为 16 (原版 14)
- 增加 `ZKsync` 相关的网络配置，删除 rinkby, kovan, geoli 等已经废弃的测试网络
- 增加 ZKsync Sepolia 网络上部署的 Uniswap V3 合约地址
- 增加 ZKsync Sepolia 网络上的一些测试 token 的配置
  - 如果你不想自己部署测试用的 ERC20 token，可以直接用着两个 test token，他们都有无限制的 mint 函数，在区块浏览器直接 mint 即可
  - [DLD](https://sepolia.explorer.zksync.io/address/0x264d10475eF47cFABdD3A0592d285ac612A4586D)
  - [DLZT](https://sepolia.explorer.zksync.io/address/0x0581364e148898c641D7741094bC9123F5Cb959F)
- 由于我们并未部署后端路由服务，所以要关闭相应的设置
- `src/hooks/useClientSideV3Trade.ts` 在 ZKsync 网络上使用 `QuoterV2`, 因为我们并未部署 `QuoterV1`

  ```ts
  // src/hooks/useClientSideV3Trade.ts
  const useQuoterV2 = useMemo(
    () => Boolean(chainId && (isCelo(chainId) || isZksyncChainId(chainId))),
    [chainId]
  );
  ```

- `src/hooks/usePools.ts` 由于 zksync 的 create2 计算地址的方法不同，所以我们需要单独实现一个计算 pool address 的逻辑; 简单来说是在对 token 地址排序之后，使用新的
  `POOL_INIT_CODE_HASH_ZKSYNC`, `CONSTRUCTOR_INPUT_HASH` 和 prefix 计算 pool address

  ```ts
  const POOL_INIT_CODE_HASH_ZKSYNC =
    "0x010013f177ea1fcbc4520f9a3ca7cd2d1d77959e05aa66484027cb38e712aeed";
  const CONSTRUCTOR_INPUT_HASH =
    "0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470";

  export function computePoolAddressZksync({
    factoryAddress,
    tokenA,
    tokenB,
    fee,
  }: {
    factoryAddress: string;
    tokenA: Token;
    tokenB: Token;
    fee: FeeAmount;
  }): string {
    const [token0, token1] = tokenA.sortsBefore(tokenB)
      ? [tokenA, tokenB]
      : [tokenB, tokenA]; // does safety checks
    const salt = keccak256(
      ["bytes"],
      [
        defaultAbiCoder.encode(
          ["address", "address", "uint24"],
          [token0.address, token1.address, fee]
        ),
      ]
    );
    const prefix = keccak256(["bytes"], [toUtf8Bytes("zksyncCreate2")]);
    const addressBytes = keccak256(
      ["bytes"],
      [
        concat([
          prefix,
          zeroPad(factoryAddress, 32),
          salt,
          POOL_INIT_CODE_HASH_ZKSYNC,
          CONSTRUCTOR_INPUT_HASH,
        ]),
      ]
    ).slice(26);
    return getAddress(addressBytes);
  }
  ```

详细的代码修改查看这里 [Comparing changes](https://github.com/Uniswap/interface/compare/v4.160.0...0x-stan:uniswap-zksync-interface:zksync)

接下来我们看看 interface 的业务流程，以 `addLiquidity` 和 `swap` 为例

### add liquidity flow

![uniswap-v3-add-liquidity.drawio](./docs/img/uniswap-v3-add-liquidity.svg)

分为 3 个角色

- `LP provider` (也就是用户)
- `Web Client` (即 interface)
- `SmartContract`

当用户使用 interface 添加流动性，流程大致如下：

- 判断 position 是否已存在，通常根据 url 中的参数判断，如果用户追加流动性，url 参数中会传入 position NFT 的 tokenId

  ```ts
  // src/pages/AddLiquidity/index.tsx
  const {
    currencyIdA,
    currencyIdB,
    feeAmount: feeAmountFromUrl,
    tokenId,
  } = useParams<{
    currencyIdA?: string;
    currencyIdB?: string;
    feeAmount?: string;
    tokenId?: string;
  }>();
  ```

- 如果 position 已存在，通过 `useV3useV3PositionFromTokenId` hook 函数，向 Manager 合约请求 position 相关信息；如果不存在，这里将 `undefined` 传递给下一个函数

  ```ts
  // check for existing position if tokenId in url
  // call NonfungiblePositionManager contract `positions(uint256 tokenId)`
  const { position: existingPositionDetails, loading: positionLoading } =
    useV3PositionFromTokenId(tokenId ? BigNumber.from(tokenId) : undefined);
  ```

- `useDerivedPositionInfo` hook 函数根据上一步传入的信息，返回 `class Position` (`@uniswap/v3-sdk`); 如果上一步传入 undefinded， 则返回 undefined

- `useV3DerivedMintInfo` hook 函数会处理很多核心逻辑，根据用户的输入，准备调用交易的calldata
  - 处理用户的输入 (position的相关参数)

    ```ts
    // src/state/mint/v3/hooks.tsx
    const { independentField, typedValue, leftRangeTypedValue, rightRangeTypedValue, startPriceTypedValue } =
    useV3MintState()
    ```

  - `useCurrencyBalances` hook 函数会请求相应的 ERC20 合约 `balanceOf` 接口，查看用户的 token 余额
  - `usePool` hook 函数，会计算 pool address，并向链上请求数据，并包装成 `class Pool` (`@uniswap/v3-sdk`) 对象返回给我们
    - `PoolCache.getPoolAddress` 根据配置的两个 token 的address，依据 create2 机制计算 pool address，具体原理参考 [#getPoolAddress](#getPoolAddress)
    - 如果 Pool 已存在，向 `UniswapV3Pool` 合约请求 `slot0()` 和 `liquidity()` 接口，前者包含当前价格和tick等信息，后者是公式中的 `L`;
    - 根据请求得到的信息，包装 `class Pool` (`@uniswap/v3-sdk`)， 如果 Pool 还未创建，则返回 null
  - 如果 Pool 合约还未创建，生成 MockPool 对象，此时需要用户输入 `init price`
  - 根据用户输入的价格区间 (`leftRangeTypedValue`, `rightRangeTypedValue`)，生成 ticks 对象 ( lower 和 upper 的tick index)
  - 根据用户输入的token数量 `independentAmount`，计算另一个token所需要的数量 `dependentAmount`;
    - 这里使用的是 `@uniswap/v3-sdk` 的 `Position.fromAmount0() / Position.fromAmount1()`
    - 原理可以参考这篇文章 [Uniswap v3 详解（二）：创建交易对/提供流动性](https://paco0x.org/uniswap-v3-2)

    ```ts
    const dependentAmount: CurrencyAmount<Currency> | undefined = useMemo(() => {
      ...

      const position: Position | undefined = wrappedIndependentAmount.currency.equals(poolForPosition.token0)
        ? Position.fromAmount0({
            pool: poolForPosition,
            tickLower,
            tickUpper,
            amount0: independentAmount.quotient,
            useFullPrecision: true, // we want full precision for the theoretical position
          })
        : Position.fromAmount1({
            pool: poolForPosition,
            tickLower,
            tickUpper,
            amount1: independentAmount.quotient,
          })

      const dependentTokenAmount = wrappedIndependentAmount.currency.equals(poolForPosition.token0)
        ? position.amount1
        : position.amount0
      return dependentCurrency && CurrencyAmount.fromRawAmount(dependentCurrency, dependentTokenAmount.quotient)
    }, [
      independentAmount,
      outOfRange,
      dependentField,
      currencyB,
      currencyA,
      tickLower,
      tickUpper,
      poolForPosition,
      invalidRange,
    ])
    ```

  - 最终 `useV3DerivedMintInfo` hook 将结合合约返回和用户输入，计算出所有发送交易所需的字段

- 在用户点击 Add 按钮时，`onAdd` 函数会生成 calldata，并发送交易
  - 根据 Position 是否存在，将调用 Manager 合约的不同函数 `mint()` 或者 `increaseLiquidity()`，两者所需的参数不同，所以需要分情况对待
  - 生成calldata， 发送交易

#### getPoolAddress

Uniswap 使用了 create2 机制来创建 Pool 合约，所以在 Pool 创建之前，我们可以在链下提前计算好 Pool Address。

主网的 create2 的计算机制如下：

- `keccak256(concat([ "0xff", from, salt, initCodeHash ]))`

而 ZKsync Era create2 的计算规则有一点区别

- `keccak256(concat([ keccak256("zksyncCreate2"), from, salt, initCodeHash ]))`

另外需要注意的是 salt 的计算要跟 factory 合约保持一致

- `keccak256(concat([token0.address, token1.address, fee]))`

```ts
// src/hooks/usePools.ts
static getPoolAddress(chainId: number, factoryAddress: string, tokenA: Token, tokenB: Token, fee: FeeAmount): string {
  ...
  const address = {
    key,
    address: isZksyncChainId(chainId)
      ? computePoolAddressZksync({
          factoryAddress,
          tokenA,
          tokenB,
          fee,
        })
      : computePoolAddress({
          factoryAddress,
          tokenA,
          tokenB,
          fee,
        }),
  }
  this.addresses.unshift(address)
  return address.address
}

function computePoolAddressZksync({
  factoryAddress,
  tokenA,
  tokenB,
  fee,
}: {
  factoryAddress: string
  tokenA: Token
  tokenB: Token
  fee: FeeAmount
}): string {
  const [token0, token1] = tokenA.sortsBefore(tokenB) ? [tokenA, tokenB] : [tokenB, tokenA] // does safety checks
  const salt = keccak256(
    ['bytes'],
    [defaultAbiCoder.encode(['address', 'address', 'uint24'], [token0.address, token1.address, fee])]
  )
  const prefix = keccak256(['bytes'], [toUtf8Bytes('zksyncCreate2')])
  const addressBytes = keccak256(
    ['bytes'],
    [concat([prefix, zeroPad(factoryAddress, 32), salt, POOL_INIT_CODE_HASH_ZKSYNC, CONSTRUCTOR_INPUT_HASH])]
  ).slice(26)
  return getAddress(addressBytes)
}

// v3-sdk/src/utils/computePoolAddress.ts
import { getCreate2Address } from '@ethersproject/address'
function computePoolAddress() {
  ...
  return getCreate2Address(
    factoryAddress,
    keccak256(
      ['bytes'],
      [defaultAbiCoder.encode(['address', 'address', 'uint24'], [token0.address, token1.address, fee])]
    ),
    initCodeHashManualOverride ?? POOL_INIT_CODE_HASH
  )
}
```

### swap flow

![uniswap-v3-swap.drawio](./docs/img/uniswap-v3-swap.svg)

- `useSwapState` hook 函数处理用户的输入 `independentField`,`typedValue`,`recipient`
- `useDerivedSwapInfo` hook 函数根据用户的输入参数，请求链上数据和计算处理
  - `useENS` hook 函数会请求 `ENS Registry / Resolver` 合约，处理 recipient (如果输入是 ens)
  - `useCurrencyBalances` hook 函数会请求相关 ERC20 合约，查询余额
  - `isExactIn` 根据用户输入的行为判断是置顶 amountIn 还是 amountOut
  - `allowedSlippage` 根据用户输入的最大滑点限制生成，一般使用默认值
  - `useBestTrade` hook 函数返回最佳的交易路径，通过后端api接口和 SwapRouter 合约请求结果综合判断
    - `useRoutingAPITrade` hook 请求后端服务接口返回最佳交易路径；由于我们并没有部署相关后端服务，所以这里不会有返回结果
    - `useClientSideV3Trade` hook 先计算所有可用的 Pool，生成相应的calldata，然后调用 QuoteV2 合约预估交易的结果，最终筛选出最优的交易路径
      - `useAllV3Routes` hook 根据 tokenA, tokenB 的地址以及所有可选的 fee ，列出所有 Pool Address，并检查是否有流动性，返回所有可用的 Pool
      - 生成对应的calldata，请求 `QuoteV2` 合约，预估交易结果 `quotesResults`； Quote 合约是用于预估交易结果的合约，并不会执行交易；
      - 根据预估结果，筛选出最优的交易路径，即 滑点+手续费 最少
- approve 阶段，分为两种方式
  - `useERC20PermitFromTrade` hook 函数生成授权approve的签名，如果 Token 合约支持 EIP2612 标准，默认使用该方法
  - 如果 Token 不支持 EIP2612， 则需要单独发起 approve 交易，授权
- 根据最优交易路径的calldata发起swap交易，调用 `SwapRouter` 的 `exactInput()` 或 `exactOutput()` 函数

## Reference

- <https://github.com/uniswap-zksync/era-uniswap-deploy-v3>
- <https://github.com/0x-stan/uniswap-zksync-interface>
- <https://github.com/uniswap-zksync/era-uniswap-v3-core>
- <https://github.com/uniswap-zksync/era-uniswap-deploy-v3>
- <https://github.com/uniswap-zksync/era-uniswap-swap-router-contracts>
- <https://github.com/uniswap-zksync/era-uniswap-v3-staker>
- <https://github.com/uniswap-zksync/era-openzeppelin-contracts>
