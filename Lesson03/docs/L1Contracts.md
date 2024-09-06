# L1 Contracts

## Diamond Contract（钻石合约）

钻石合约（Diamond Contract, 又称 State Transition Contract），是一种特殊的智能合约结构，它允许单个合约包含多个逻辑模块，每个模块都被称为“Facet”。这种设计极大地增强了合约的灵活性和可扩展性，使得我们可以在不需要部署新的合约的情况下，动态地添加或修改功能。

![Diamond-scheme.png](./docs/img/Diamond-scheme.png)

钻石合约的核心优势在于：

- 模块化设计：通过将不同功能模块化，可以有效管理和扩展合约的功能。
- 节省 Gas 费用：在扩展合约功能时，无需重新部署整个合约，从而节省了 Gas 费用。

### DiamondProxy

在钻石合约中，DiamondProxy 是一个关键组件，它负责将外部调用代理到正确的 Facet 模块。`DiamondProxy` 的结构允许我们通过更新一个存储在合约中的地址来动态地改变功能指向的逻辑模块，从而实现灵活的功能扩展和维护。

通过 `delegatecall` 操作将调用委派到特定的 Facet 模块，这使得主合约可以动态地更新或添加新功能而无需改变其接口或地址。

- 详细讨论 DiamondProxy 的实现细节

> 代理合约模式适用于简单的合约升级，而钻石合约模式则提供更强的模块化和灵活性，适合复杂的合约系统。

ZKsync Era 钻石合约的几个模块包括：

1. DiamondProxy: 这是钻石合约的代理模块，负责接收外部调用并将其委托给正确的功能模块（Facet）进行处理。
2. AdminFacet: 负责管理钻石合约的各个模块，主要包括合约的升级、模块的添加和删除等操作。
3. MailboxFacet 处理 L1 <-> L2 的通信
4. ExecutorFacet: 执行特定的系统交易和操作逻辑，确保在 ZKsync 系统中高效处理交易。
5. GettersFacet: 独立的 facet，它的唯一功能是提供 `view` 和 `pure` 方法
6. GovernanceFacet: 与治理相关的功能模块，负责合约的治理操作，如投票和提案处理等。
7. ValidatorFacet: 功能：涉及验证逻辑，用于检查和验证关键的操作和状态。这一模块是确保系统安全性的重要部分。

## AdminFacet

这个 facet 负责配置设置和可升级性，处理的任务包括：

- **特权地址管理**：更新关键角色，包括 governor 和验证者。
- **系统参数配置**：调整关键系统设置，例如 L2 bootloader 字节码哈希、验证者地址、验证者参数、费用配置等。
- **冻结能力**：在升级期间或响应检测到的漏洞时，通过 diamond proxy 执行 facets 的冻结/解冻操作，以保障生态系统的安全。

对 AdminFacet 的控制分为两个主要实体：

- **STM (State Transition Manager)**：一个独立的智能合约，可以对系统进行关键性更改作为协议升级的一部分。尽管目前只有一个版本的 STM 存在，但该架构允许通过后续升级引入未来版本。STM 的控制权由 `Governance.sol` 合约和 Admin 实体（见下文）共享。`Governance.sol` 由两个 multisig 控制：Admin multisig（见下文）和 Security council multisig（加密领域的受人尊敬的贡献者）。合作完成后，这些实体有权执行即时升级，而 Matter Labs 仅限于调度升级。

- **Admin**：由 Matter Labs 管理的 multisig 智能合约，可以对系统执行非关键性更改，例如授予验证者权限。需要注意的是，Admin 是唯一拥有治理权的 multisig。

## MailboxFacet

处理 L1 <-> L2 的通信。

Mailbox 执行以下三个功能：

- **L1 <-> L2 Communication**：支持数据和交易请求从 L1 发送到 L2，反之亦然，支持多层协议的实现。
- **Bridging Native Tokens**：允许将 ether 或 ERC20 代币桥接到 L2，使用户能够在 L2 生态系统中使用这些资产。
- **Censorship Resistance Mechanism**：目前处于研究阶段。

L1->L2 的通信通过在 L1 上请求 L2 交易并在 L2 上执行来实现。这意味着用户可以调用 L1 合约上的函数来保存关于交易的信息到某个队列中。稍后，验证者可以在 L2 上处理它，并在 L1 优先队列中标记为已处理。目前，它用于从 L1 向 L2 发送信息或实现多层协议。用户在请求 L1->L2 交易时，以 Native token (ETH)支付交易执行费用。

**注意**：当用户从 L1 请求交易时，在 L2 上发起的交易将具有特别 `msg.sender` 。

```solidity
address sender = msg.sender;
if (sender != tx.origin) {
    sender = AddressAliasHelper.applyL1ToL2Alias(msg.sender);
}

uint160 constant offset = uint160(0x1111000000000000000000000000000000001111);

function applyL1ToL2Alias(address l1Address) internal pure returns (address l2Address) {
  unchecked {
    l2Address = address(uint160(l1Address) + offset);
  }
}
```

对于大多数的 rollups，需要地址别名来防止跨链攻击。如果我们简单地重用相同的 L1 地址作为 L2 发送者，那么这种攻击将成为可能。在 ZKsync Era 中，地址推导规则与 Ethereum 不同，因此跨链攻击已经不可能发生。然而，ZKsync Era 未来可能会增加对完整 EVM 支持，因此应用地址别名为未来的 EVM 兼容性留出了空间。

L1 -> L2 的通信也用于桥接**native tokens**。如果 native token 是 ether（在 ZKsync Era 中的情况），用户在发起 L1 合约上的交易请求时应该包括 `msg.value`。如果 native token 是 ERC20，那么合约将花费用户的配额。在 L2 上执行交易之前，指定的地址将收到资金。要提取资金，用户应调用 L2BaseToken 系统合约的 `withdraw` 函数。

L2 -> L1 与 L1 -> L2 通信相比，仅仅是基于信息的传递，而不是在 L1 上执行交易。

## ExecutorFacet

这是一个接收 L2 batch 的合约，强制执行数据可用性并检查 zk-proofs 的有效性。有关 pubdata 如何被解析和处理的更多信息，请参阅关于 pubdata post EIP-4844 和 Handling pubdata 的文档，详细说明了内容。

状态转换分为三个阶段：

- **commitBatches** - 检查 L2 batch 的时间戳，处理 L2 日志，为 batch 保存数据，并为 zk-proof 准备数据。
- **proveBatches** - 验证 zk-proof。
- **executeBatches** - 完成状态，标记 L1 -> L2 的通信处理，并使用 L2 日志保存到 Merkle 树中。

每个 L2 -> L1 系统日志将有一个键，它是以下内容的一部分：

```solidity
enum SystemLogKey {
    L2_TO_L1_LOGS_TREE_ROOT_KEY,
    TOTAL_L2_TO_L1_PUBDATA_KEY,
    STATE_DIFF_HASH_KEY,
    PACKED_BATCH_AND_L2_BLOCK_TIMESTAMP_KEY,
    PREV_BATCH_HASH_KEY,
    CHAINED_PRIORITY_TXN_HASH_KEY,
    NUMBER_OF_LAYER_1_TXS_KEY,
    BLOB_ONE_HASH_KEY,
    BLOB_TWO_HASH_KEY,
    EXPECTED_SYSTEM_CONTRACT_UPGRADE_TX_HASH_KEY
}
```

当一个 batch 被提交时，我们处理 L2 -> L1 系统日志。以下是这些日志中所期待的内容不变量：

- 在一个给定的 batch 中，将有 9 或 10 个系统日志。第 10 个日志仅在协议升级时需要。
- 每个包含在 `SystemLogKey` 中的键将有一个日志。
- 来自 `L2_TO_L1_MESSENGER` 的三个日志，包含以下键：
  - `L2_TO_L1_LOGS_TREE_ROOT_KEY`
  - `TOTAL_L2_TO_L1_PUBDATA_KEY`
  - `STATE_DIFF_HASH_KEY`
- 来自 `L2_SYSTEM_CONTEXT_SYSTEM_CONTRACT_ADDR` 的两个日志，包含以下键：
  - `PACKED_BATCH_AND_L2_BLOCK_TIMESTAMP_KEY`
  - `PREV_BATCH_HASH_KEY`
- 来自 `L2_PUBDATA_CHUNK_PUBLISHER_ADDR` 的两个日志，包含以下键：
  - `BLOB_ONE_HASH_KEY`
  - `BLOB_TWO_HASH_KEY`
- 来自 `L2_BOOTLOADER_ADDRESS` 的两个或三个日志，包含以下键：
  - `CHAINED_PRIORITY_TXN_HASH_KEY`
  - `NUMBER_OF_LAYER_1_TXS_KEY`
  - `EXPECTED_SYSTEM_CONTRACT_UPGRADE_TX_HASH_KEY`
- 没有来自其他地址的日志（可能在未来会发生变化）。

## DiamondInit

它是一个只有一个 funciont 的合约，实现了初始化钻石代理的逻辑。它仅在钻石合约的构造函数上调用一次，并且不会作为 Facet 保存。

## ValidatorTimelock

ValidatorTimelock 是一个包含时间锁机制的合约，用于确保在对验证者集（Validator Set）进行更改时有足够的时间进行审核和反应。这种机制可以防止突然或未经审查的更改对网络的安全性产生不利影响。

在这种机制下，变更不会立即生效，而是会在一段时间后才会执行。这个时间段允许社区成员、利益相关者和验证者本身对即将发生的更改进行充分讨论和评估。这种机制可以有效防止恶意行为者利用紧急更改来破坏系统的安全性或稳定性。

ValidatorTimelock 的实现通常包含以下关键要素：

- 时间锁定期：规定了从提议变更到变更实际生效之间的延迟时间。
- 执行条件：确保在时间锁定期满之前，变更不能被执行。
- 安全措施：防止在时间锁定期内对变更进行未授权的修改或取消。

## GettersFacet

一个单独的 facet，它的唯一功能是提供 `view` 和 `pure` 方法。它还实现了 `diamond loupe`，这使得管理 facets 变得更加容易。此合约绝不能被冻结。
