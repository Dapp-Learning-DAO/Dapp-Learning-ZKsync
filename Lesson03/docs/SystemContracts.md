# 系统合约

虽然大多数原始的 EVM 操作码可以直接支持（例如，零值调用、加法/乘法/内存/存储管理等），但有些操作码并未被虚拟机默认支持，它们通过“系统合约”实现——这些合约位于特殊的内核空间（kernel space），即地址空间范围为 `[0..2^16-1]`，并且它们具有一些特殊权限，这是用户合约所不具备的。这些合约在创世区块时预先部署，并且只能通过系统升级从 L1 层进行代码更新。

下文将解释每个系统合约的使用。

关于系统合约的实现细节和执行要求的大部分内容，可以在它们各自代码库的文档注释中找到。本章节仅作为这些合约的概览。

## SystemContext

此合约用于支持 VM 默认不包括的各种系统参数，例如 `chainId`、`origin`、`ergsPrice`、`blockErgsLimit`、`coinbase`、`difficulty`、`baseFee`、`blockhash`、`block.number`、`block.timestamp`。

需要注意的是，在创世区块，系统合约的构造函数不会运行，即常量上下文值会在创世时明确设置。值得注意的是，如果未来我们想要升级合约，我们将通过 `ContractDeployer` 进行操作，这样构造函数就会被运行。

此合约还负责确保批次、L2 区块和虚拟区块的有效性和一致性。实现本身相当简单，但为了更好地理解此合约，详见[block processing](https://docs.zksync.io/build/developer-reference/batches-and-l2-blocks)页面。

## AccountCodeStorage

账户的代码哈希存储在此合约的存储中。每当 VM 调用具有地址 `address` 的合约时，它会检索该系统合约存储槽 `address` 下的值，如果该值非零，它会将此值用作账户的代码哈希。

每当合约被调用时，VM 会要求操作员提供账户代码哈希的原像。这就是为什么代码哈希的数据可用性至关重要的原因。

## Constructing vs Non-Constructing Code Hash

为了防止合约在其构造期间被调用，我们设置了标记（即账户字节码哈希的第二个字节）为 `1`。这样，VM 将确保每当在没有 `isConstructor` 标志的情况下调用合约时，默认账户（即 EOA）的字节码将替代原始字节码。

## BootloaderUtilities

此合约包含了一些仅用于引导加载器功能的方法，这些方法被从引导加载器中移出，以方便在编写此逻辑时避免使用 Yul。

## DefaultAccount

每当一个合约同时不满足以下两个条件时：

- 不属于内核空间（kernel space）
- 没有部署任何代码（`AccountCodeStorage` 中对应存储槽的值为零）

将使用默认账户的代码。该合约的主要目的是为调用它的钱包用户和合约提供类似 EOA 的体验，即它不应与 Ethereum 上的 EOA 账户有可辨别的区别（除了消耗的 gas 外）。

默认账户抽象的实现。这段代码是默认情况下用于所有不在内核空间且没有部署合约的地址。此地址：

- 包含我们账户抽象协议的最小实现。请注意，它支持内置的支付主协议流（built-in paymaster flows）。
- 当任何人（引导加载器除外）调用它时，它的行为方式与调用 EOA 相同，即它总是返回 `success = 1`，`returndatasize = 0`，对于引导加载器以外的任何调用都是如此。

## Ecrecover

`Ecrecover` 预编译的实现。由于预期会频繁使用，因此使用纯 Yul 编写并采用自定义的内存布局。

该合约接受与 EVM 预编译相同格式的 `calldata`，即：前 32 字节是哈希值，接下来的 32 字节是 `v`，再接下来的 32 字节是 `r`，最后的 32 字节是 `s`。

它还按照 EVM 预编译的相同规则验证输入：

- `v` 应该是 27 或 28，
- `r` 和 `s` 应该小于曲线阶。

之后，它进行预编译调用，如果调用失败，则返回空字节；如果成功，则返回恢复的地址。

## Empty contracts

某些合约需要具有类似 EOA 的行为，即它们可以始终被调用并返回成功值。例如，地址为 0 的合约就是这种情况。我们还要求引导加载器可调用，以便用户可以向其转移 ETH。

对于这些合约，我们在创世时插入了 `EmptyContract` 代码。它基本上是一个无操作代码，只返回 `success=1`。

## SHA256 & Keccak256

请注意，与以太坊不同，Keccak256 在 ZKsync 上是一个预编译（不是操作码）。

这些系统合约作为各自加密预编译实现的包装器。预计它们会被频繁使用，尤其是 Keccak256，因为 Solidity 使用它来计算映射和动态数组的存储槽。这也是为什么我们使用纯 Yul 编写这些合约。

系统合约接受输入并将其转换为 zk 电路预期的格式。通过这种方式，一部分工作从加密转移到了智能合约，后者更易于审计和维护。

两个合约都应该根据各自的规范对输入进行填充，然后使用填充后的数据进行预编译调用。所有其他哈希工作都将在 zk 电路中完成。重要的是要注意，预编译的加密部分期望处理填充后的数据。这意味着在应用填充时的错误可能导致交易无法证明。

## L2BaseToken & MsgValueSimulator

与以太坊不同，zkEVM 没有任何特殊原生代币的概念。这就是为什么我们需要通过两个合约来模拟以太币的操作：`L2BaseToken` 和 `MsgValueSimulator`。

`L2BaseToken` 是一个持有用户 ETH 余额的合约。该合约不提供 ERC20 接口。转移以太币的唯一方法是 `transferFromTo`。它允许一些系统合约代表用户进行转账。这是为了确保接口尽可能接近以太坊，即转移以太币的唯一方法是通过调用具有一定 `msg.value` 的合约。这就是 `MsgValueSimulator` 系统合约的作用。

每当有人想要进行非零值调用时，他们需要调用 `MsgValueSimulator` 并传递以下内容：

- 与原始调用相同的 `calldata`。
- 传递 `value`，以及该调用是否应在第一个额外 ABI 参数中标记为 `isSystem`。
- 在第二个额外 ABI 参数中传递被调用者的地址。

## KnownCodeStorage

此合约用于存储某个代码哈希是否“已知”，即可以用于部署合约。在 ZKsync 上，L2 只存储合约的代码哈希，而不存储代码本身。因此，协议必须确保不会部署任何带有未知字节码（即哈希值没有已知前像）的合约。

用户为每个交易提供的工厂依赖字段包含了合约字节码哈希的列表，这些哈希将被标记为已知。我们不能简单地信任操作员将这些字节码哈希标记为“已知”，因为操作员可能存在恶意行为并隐藏前像。我们通过以下方式确保字节码的可用性：

- 如果交易来自 L1，即其所有工厂依赖项已经在 L1 上发布，我们可以简单地将这些依赖项标记为“已知”。
- 如果交易来自 L2（即工厂依赖项尚未在 L1 上发布），我们要求用户支付与字节码长度成比例的 gas 费用。之后，我们在 L2→L1 日志中发送合约的字节码哈希。L1 合约有责任验证相应的字节码哈希是否已在 L1 上发布。

`ContractDeployer` 系统合约有责任仅部署那些已知的代码哈希。

`KnownCodesStorage` 合约还负责确保所有“已知”字节码哈希也是有效的。

## ContractDeployer & ImmutableSimulator

`ContractDeployer` 是一个系统合约，负责在 ZKSync 上部署合约。要更好地理解它的工作原理，需要结合合约部署在 ZKSync 上是如何工作的。与以太坊不同，在以太坊中 `create` / `create2` 是操作码，而在 ZKSync 上，这些是通过调用 `ContractDeployer` 系统合约实现的。

为了增加安全性，我们还区分了普通合约和账户的部署。因此，用户将使用的主要方法是 `create`、`create2`、`createAccount` 和 `create2Account`，它们分别模拟了 CREATE 和 CREATE2 的行为，用于部署普通合约和账户合约。

- [ContractDeployer interface](https://github.com/matter-labs/era-contracts/blob/main/system-contracts/contracts/interfaces/IContractDeployer.sol)

## Address derivation

每个支持 L1→L2 通信的 Rollup 都需要确保 L1 和 L2 上的合约地址在通信期间不会重叠（否则，L1 上的某些恶意代理可能会更改 L2 合约的状态）。通常，Rollup 通过以下两种方式解决此问题：

1. 在 L1→L2 通信期间，对地址进行某种形式的异或（XOR）/加法（ADD）常量操作。这就是 Rollup 更接近完整 EVM 等效性解决方案的方法，因为它允许它们在 L1 上保持相同的推导规则，但代价是 L1 上的合约账户必须在 L2 上重新部署。
2. 使用不同于以太坊的推导规则。这是 ZKSync 选择的方法，主要是因为我们的字节码与 EVM 不同，CREATE2 地址推导在实践中也会有所不同。

你可以在 `ContractDeployer` 的 `getNewAddressCreate2` 和 `getNewAddressCreate` 方法中查看我们的地址推导规则。

请注意，我们仍然在 L1→L2 通信期间向地址添加了某些常量，以便将来我们可以在某种程度上支持 EVM 字节码。

## Deployment nonce

在以太坊上，相同的 nonce 用于 CREATE 操作来创建账户和 EOA 钱包。而在 ZKSync 上并非如此，我们使用一个单独的 nonce，称为 "deploymentNonce"，来跟踪账户的 nonce。这主要是为了与自定义账户保持一致，并在未来支持多重调用功能。

## General process of deployment

- 在递增部署 nonce 之后，合约部署器必须确保要部署的字节码是可用的。
- 然后，它将字节码哈希与一个特殊的构造标记一起作为即将部署的合约地址的代码。
- 接着，如果在调用中传递了任何值，合约部署器会将其传递给已部署的账户，并将 `msg.value` 设置为下一个等于该值。
- 然后，它使用 `mimic_call` 以账户的名义调用合约的构造函数。
- 它解析构造函数返回的不可变数组（我们稍后会详细讨论不可变变量）。
- 调用 `ImmutableSimulator` 来设置用于已部署合约的不可变变量。

请注意，它与 EVM 方法的不同之处：在 EVM 上，当合约部署时，它执行 initCode 并返回 deployedCode。而在 ZKSync 上，合约只有部署代码，并且可以将不可变变量设置为由构造函数返回的存储变量。

## Constructor

在以太坊上，构造函数只是 initCode 的一部分，它在合约部署期间执行，并返回合约的部署代码。在 ZKSync 上，部署代码和构造函数代码之间没有分离。构造函数始终是部署代码的一部分。为了防止它被调用，编译器生成的合约仅在提供 `isConstructor` 标志时才调用构造函数（它仅适用于系统合约）。

执行后，构造函数必须返回一个数组：

```solidity
struct ImmutableData {
    uint256 index;
    bytes32 value;
}
```

基本上表示传递给合约的不可变数据的数组。

## Immutables

不可变数据存储在 ImmutableSimulator 系统合约中。每个不可变数据的 index 的定义方式是编译器规范的一部分。该合约只是简单地将每个特定地址的索引映射到值。

每当合约需要访问某些不可变数据的值时，它们会调用 `ImmutableSimulator.getImmutable(getCodeAddress(), index)`。请注意，在 ZKSync 上，可以获取当前执行地址。

## Return value of the deployment methods

如果调用成功，则返回已部署合约的地址。如果部署失败，则会向上传播错误。

## L1Messenger

一个用于从 ZKSync 向 L1 发送任意长度 L2→L1 消息的合约。虽然 ZKSync 原生支持的 L1→L2 日志数量相当有限，一次只能传输大约 64 字节的数据，但我们通过以下方法允许发送几乎任意长度的 L2→L1 消息：

L1 Messenger 接收消息，将其哈希并仅发送其哈希值以及通过 L2→L1 日志发送的原始发送者地址。然后，L1 智能合约有责任确保操作员在批处理的承诺中提供了该哈希的完整前像。

`L1Messenger` 还负责验证要在 L1 上发送的全部公有数据。

- [L1Messenger interface](https://github.com/matter-labs/era-contracts/blob/main/system-contracts/contracts/interfaces/IL1Messenger.sol)

## NonceHolder

为我们的账户提供 nonce 的存储功能。除了使操作员更容易对交易进行排序（即通过读取账户的当前 nonce），它还具有另一个独立的目的：确保地址和 nonce 的组合始终是唯一的。

它提供了一个函数 `validateNonceUsage`，引导加载器使用该函数检查某个 nonce 是否已被某账户使用。引导加载器在交易的验证步骤之前强制要求 nonce 标记为未使用，并在之后将其标记为已使用。合约确保一旦标记为已使用，nonce 不能再恢复到“未使用”状态。

注意，nonce 不一定必须是单调递增的（这是为了支持更有趣的账户抽象应用，例如可以自行启动交易的协议、类似 Tornado Cash 的协议等）。这就是为什么有两种方法可以将某个 nonce 设置为“已使用”：

- 通过递增账户的 `minNonce`（从而使所有低于 `minNonce` 的 nonce 标记为已使用）。
- 通过 `setValueUnderNonce` 在 nonce 下设置某个非零值。这样，该键将被标记为已使用，并且不再允许用作账户的 nonce。这种方式也相当高效，因为这 32 字节可以用于存储一些有价值的信息。

账户在创建时还可以指定他们希望的 nonce 排序类型：顺序（即预期 nonce 会一个一个地递增，就像 EOA 一样）或任意（nonce 可以具有任何值）。这种排序不会以任何方式由系统合约强制执行，但它更多地是对操作员在 mempool 中应如何排序交易的建议。

- [NonceHolder interface](https://github.com/matter-labs/era-contracts/blob/main/system-contracts/contracts/interfaces/INonceHolder.sol)

## EventWriter

一个负责发出事件的系统合约。

它在 `0-th extra abi data` 参数中接受主题的数量。在 `extraAbiParams` 的其余部分中，它接受要发出的事件的主题。注意，实际上事件的第一个主题包含账户的地址。通常，用户不应直接与此合约交互，而应通过 Solidity 的 `emit` 语法发出新事件。

## Compressor

对于 Rollup 来说，数据可用性是最昂贵的资源之一，因此为了降低用户的成本，我们通过以下几种方式压缩已发布的公有数据：

- 我们压缩已发布的字节码。
- 我们压缩状态差异（state diffs）。

该合约包含一些实用方法，用于验证字节码或状态差异压缩的正确性。你可以在相应的[文档](链接)中阅读更多关于我们如何压缩状态差异和字节码的信息。

## 受保护访问的一些系统合约

某些系统合约会影响账户，这在以太坊上可能不会被预期。例如，在以太坊上，EOA 增加其 nonce 的唯一方法是发送交易。此外，发送交易只能使 nonce 增加 1。在 ZKSync 上，nonce 通过 `NonceHolder` 系统合约实现，如果天真地实现，用户可能会被允许通过调用此合约来增加其 nonce。这就是为什么对 `nonce holder` 的大多数非视图方法的调用仅限于带有特殊 `isSystem` 标志的调用，以便重要系统合约的交互可以由账户开发者有意识地管理。

同样的情况也适用于 `ContractDeployer` 系统合约。这意味着，例如，你需要明确允许你的用户部署合约，就像在 [`DefaultAccount`](https://github.com/matter-labs/era-contracts/blob/6250292a98179cd442516f130540d6f862c06a16/system-contracts/contracts/DefaultAccount.sol#L125)中那样。
