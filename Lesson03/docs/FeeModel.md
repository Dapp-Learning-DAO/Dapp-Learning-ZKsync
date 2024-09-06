# Fee Model

## Main differences from EVM

ZKsync 和其他 L2 都面临一个问题，即无法轻易采用与以太坊相同的 fee 模型。主要原因是需要在以太坊上发布 `pubdata`。这意味着 L2 交易的价格将取决于波动的 L1 gas 价格，并且不能简单地硬编码。

此外，ZKsync 作为一个 zkRollup 需要使用 `zero-knowledge proofs` 来证明每一个操作。这带来了一些细微的差异。

## Different opcode pricing

在 ZKsync 上的操作在 `zero-knowledge proof` 条件下的“复杂性”/“定价”往往与标准 CPU 术语下的不同。例如，`keccak256` 被优化用于 CPU 性能，但证明成本更高。

因此 gas 价格与以太坊上的有很大不同。

## I/O pricing

在以太坊上，每当第一次读取/写入存储槽时，会为首次访问该槽的操作收取一定量的 gas。类似机制用于账户：每当第一次访问账户时，都会为读取账户数据收取一定量的 gas。在 EVM 上，账户的数据包括其 `nonce`、余额和代码。ZKsync 使用类似的机制，但有一些区别。

### Storage costs

就像 EVM 一样，ZKsync 也支持“warm”和“cold”存储槽。然而，流程略有不同：

1. 首先向用户收取（最大）冷存储槽成本。
2. Operator 被要求进行退款。
3. 然后，退款直接返还给用户。

不像 EVM，用户始终需要有足够的 gas 以应对最坏情况（即使存储槽是“warm”）。此外，退款的控制目前仅由 Operator 执行，而不是由电路控制。

### Code decommitment and account access costs

与 EVM 不同的是，ZKsync 的存储并不将账户余额、`nonce` 和字节码耦合在一起。余额、`nonce` 和代码哈希是使用标准存储变量的三个单独存储项。而访问字节码时使用了一种不同的方法。

ZKsync 称字节码解压过程为 `code decommitment`，因为这是将代码承诺（即版本代码哈希）转换为其原象(preimage)的过程。每当调用具有特定代码哈希的合约时，将执行以下逻辑：

1. Operator 被询问这是否是第一次进行代码解压。
2. 如果 Operator 回答 “yes”，那么用户将支付全部费用。否则，用户无需为解压支付费用。
3. 如有必要，代码将解压到 [`code page`](https://docs.zksync.io/zk-stack/components/compiler/specification/binary-layout#memory)（EraVM中一个特定的内存区域）。

与存储交互不同，该过程的正确性部分由电路强制执行，即如果达到步骤 (3)，代码被解压，将证明 Operator 在步骤 (1) 中的回答是正确的。但是，如果程序在步骤 (2) 中耗尽 gas，则无法证明第一个陈述的正确性。原因是在调用 `decommitment` 时，在电路中很难证明这是否确实是第一次解压操作。

对于诚实的 Operator 来说，这种方法提供了更好的用户体验，因为不需要事先支付全部费用。

### Conclusion

总结一下，ZKSync Era 支持类似于 EVM 的“cold”/“warm”机制，但目前这些机制仅由 Operator 强制执行，即应用程序的用户不应依赖这些机制。只要用户有足够的 gas 来支付最坏情况下的费用，即“cold”场景，执行的正确性就有保证。

### Memory pricing

ZKSync Era 具有不同的内存定价规则：

- 每当调用用户合约时，将免费提供 `2^12` 字节的内存，然后根据字节长度收费。
- 每当调用内核空间（即系统）合约时，将免费提供 `2^21` 字节的内存，然后根据长度线性开始收费。

请注意，与 EVM 不同，ZKsync 从不使用内存扩展价格的二次分量。

### Different intrinsic costs

与以太坊不同，在以太坊中，交易的固有成本（`21000` gas）用于支付更新用户余额、`nonce` 和签名验证的费用，在 ZKSync 上这些价格不包括在交易的固有成本中，这是由于原生支持的账户抽象机制，意味着每种账户类型可能有其自己的交易成本。理论上，有些账户甚至可能使用更多 zk 友好的签名方案或其他类型的优化，以允许用户进行更便宜的交易。

话虽如此，ZKSync 交易确实存在一些小的固有成本，但它们主要用于支付与引导程序处理交易相关的成本，而这些成本不能轻易地在实时中准确测量。这些费用通过测试测量并硬编码。

### Charging for pubdata

对于用户来说，pubdata 是一个重要的成本因素。ZKSync Era 是基于 state diff 的 rollup，这意味着 pubdata 不是为交易数据发布的，而是为状态变化发布的：修改的存储槽位、已部署的字节码、L2 -> L1 消息等。这允许修改同一存储槽位多次的应用程序（如 oracles）在 L1 pubdata 上保持一个恒定的占用空间。正确地处理 state diff rollups 需要一个针对 pubdata 收费的特殊解决方案。这将在下一节中探讨。

### Batch overhead & limited resources of the batch

为了处理批次，ZKSync 团队需要支付验证批次、提交批次等的费用。处理批次还涉及一些操作成本。ZKsync 将这些值统称为“Batch overhead”。它由两部分组成：

- 验证电路的 L2 需求（用 L2 gas 表示）。
- 用于证明验证以及常规批次处理的 L1 需求（用 L1 gas 表示）。

ZKsync 通常会尽可能多地聚合交易，并且每笔交易为批次 overhead 支付的费用与这笔交易使批次更接近“封闭”的程度成比例，即封闭并准备好在 L1 上进行证明验证和提交。交易越接近封闭批次，就越多地使用批次的**有限资源**。

在以太坊上，批次 gas limit 存在的主要原因是为了保持系统的去中心化和负载低，假设存在正确的硬件，批次需要遵守的唯一要求就是时间。在 ZKSync 批次的情况下，有一些批次需要管理的有限资源：

- **时间**：与以太坊上的一样，交易打包不应花费太多时间，以提供更好的用户体验。为了表示所需的时间，ZKsync 使用 `batch gas limit`，注意它高于单个交易的 `gas limit`。
- **交易插槽**：bootloader 有一个有限数量的交易插槽，即每个批次不能处理超过一定数量的交易。

- **bootloader 的内存**：bootloader 需要存储交易的 ABI 编码，这会占用它的内存，从而填满它。实际上，这是对于自定义账户的大量 calldata/签名 的一种限制。

- **Pubdata bytes**：存储 diffs 带来的好处，即同一批次中发生的单个更改仅需要发布一次，系统仅在处理完交易后发布 batch public data。当前，ZKsync 在一个单一交易的末尾发布包含存储 diffs 和 L2->L1 消息等所有数据的批次大部分节点每笔交易有 128kb 的限制。

每笔交易都会根据它消耗上述资源的程度来按比例分摊批次 overhead。

需要注意的是，在交易执行之前，系统无法知道交易将占用多少系统资源，因此 ZKsync 需要为最坏情况收费，并在交易结束时提供退款。

### MAX_TRANSACTION_GAS_LIMIT

交易在计算上可以消耗的最大建议 gas 数量是 `MAX_TRANSACTION_GAS_LIMIT`。但是如果 Operator 信任用户，Operator 可以提供信任 gas limit（trusted gas limit），即超过 `MAX_TRANSACTION_GAS_LIMIT` 的限制，假设 Operator 知道他在做什么。在具有不同参数的 ZK Chain 的情况下，这可能会有所帮助。

### Derivation of `baseFee` and `gasPerPubdata`

在每个批次开始时，Operator 提供以下两个参数：

1. `FAIR_L2_GAS_PRICE`。该变量应表示 Operator 愿意接受的最低 L2 gas 价格。预计它将涵盖证明/执行单个 zkEVM gas 单位的成本、单个 gas 在封闭批次中的潜在贡献以及拥堵。

2. `FAIR_PUBDATA_PRICE`，即单个 pubdata 字节的价格，以 Wei 为单位。类似于上面的变量，它应涵盖发布单个字节的成本以及单个 pubdata 字节在封闭批次中的潜在贡献。

在上面的描述中，“contribution towards sealing the batch” 是指如果批次通常由某种资源（例如 pubdata）关闭，则 pubdata 价格应包含此成本。

然后，`baseFee` 和 `gasPerPubdata` 计算如下：

```solidity
baseFee := max(
    fairL2GasPrice,
    ceilDiv(fairPubdataPrice, MAX_L2_GAS_PER_PUBDATA())
)
gasPerPubdata := ceilDiv(pubdataPrice, baseFee)
```

虽然 ZKsync 在理论上收取 pubdata 的方式允许任何 `gasPerPubdata`，但某些 SDK 期望交易的 `gasLimit` 为 `uint64` 数字。ZKsync 更愿意将 `gasLimit` 保持在 JS 的安全“数字”范围内，以防某些人使用 `number` 类型表示 gas。因此，ZKsync 将 `MAX_L2_GAS_PER_PUBDATA` 绑定到每个 pubdata 字节的 `2^20` gas。选择此数字是为了确保 `MAX_L2_GAS_PER_PUBDATA * 2^32` 是安全的 JS 整数。`2^32` 部分是理论上可能用于 pubdata 计数器的最大值。尽管如此，在诚实的 Operator 下这个值实际上永远不会出现，但它是必需的，以防万一。

请注意，这意味着在高 L1 gas 价格下，总体 `gasLimit` 可能会超过 `u32::MAX`，并建议在一次交易中发布的 pubdata 不超过 `2^20` 字节。

### Recommended calculation of `FAIR_L2_GAS_PRICE / FAIR_PUBDATA_PRICE`

让我们定义以下常量：

- `BATCH_OVERHEAD_L1_GAS` - 用于批次的 L1 gas overhead（例如证明验证等）。
- `COMPUTE_OVERHEAD_PART` - 表示由于计算资源过度使用导致批次被封闭的可能性的常量。它的范围从 0 到 1。如果为 0，则计算将不依赖于批次关闭的成本。如果为 1，则每批次的 gas 限制必须涵盖关闭批次的全部成本。
- `MAX_GAS_PER_BATCH` - 每批次可以使用的最大 gas 数量。该值来源于每批次电路的限制。
- `PUBDATA_OVERHEAD_PART` - 表示由于 pubdata 过度使用导致批次被封闭的可能性的常量。它的范围从 0 到 1。如果为 0，pubdata 将不会依赖于关闭批次的成本。如果为 1，则每批次的 pubdata 限制必须涵盖关闭批次的全部成本。
- `MAX_PUBDATA_PER_BATCH` - 每批次可以使用的最大 pubdata 量。请注意，如果 `calldata` 被用作 `pubdata`，此变量不应超过 128kb。

以及以下浮动变量

- `MINIMAL_L2_GAS_PRICE` - 可接受的最低 L2 gas 价格，即应包括计算/证明的成本以及潜在的拥堵溢价。
- `PUBDATA_BYTE_ETH_PRICE` - 每字节 pubdata 的最低可接受价格，通常应等于单个 blob 字节或 calldata 字节的预期价格（取决于所采用的方法）。

1. `FAIR_L2_GAS_PRICE = MINIMAL_L2_GAS_PRICE + COMPUTE_OVERHEAD_PART * BATCH_OVERHEAD_L1_GAS / MAX_GAS_PER_BATCH`
2. `FAIR_PUBDATA_PRICE = PUBDATA_BYTE_ETH_PRICE + PUBDATA_OVERHEAD_PART * BATCH_OVERHEAD_L1_GAS / MAX_PUBDATA_PER_BATCH`

对于 L1->L2 交易，`MAX_GAS_PER_BATCH` 变量等于 `L2_TX_MAX_GAS_LIMIT`（因为该 gas 量足以在批次中发布最大数量的 pubdata）。此外，为了额外的安全性，对于 L1->L2 交易，`COMPUTE_OVERHEAD_PART = PUBDATA_OVERHEAD_PART = 1`，因为我们不确定关闭批次的具体原因。对于 L2 交易，通常 `COMPUTE_OVERHEAD_PART = 0`，因为与 L1->L2 交易不同，在攻击的情况下，Operator 可以简单地审查错误的交易或增加 `FAIR_L2_GAS_PRICE`，因此 Operator 可以使用平均值来获得更好的 UX。

### Note on operator’s responsibility

重申一下，上述公式用于保护 L1->L2 交易中的 Operator 免受恶意交易的影响。然而，对于 L2 交易，完全由 Operator 提供正确的数值。此设计为系统提供了更精细的控制（包括 Validiums，可能还有在另一个 L1 上的 Era 等）。

这种免费模式还为 Operator 提供了极高的灵活性，因此如果我们发现某个部分的收入足够，我们可以调整 `FAIR_L2_GAS_PRICE` 和 `FAIR_PUBDATA_PRICE` 的生成方式，且这就是最终结果（bootloader 端不会有进一步的强制执行）。

从长远来看，网络共识将确保这些数值在 ZKsync Era（或者我们可能会过渡到类似系统）上的正确性。

### Overhead for transaction slot and memory

我们还对批次内可以消耗的内存数量以及可以包含的交易数量进行了限制。

为了简化代码库，我们选择了以下常量：

- `TX_OVERHEAD_GAS = 10000` —— 将交易纳入批次时的 gas 开销。
- `TX_MEMORY_OVERHEAD_GAS = 10` —— 消耗一个 bootloader 内存字节的开销。

我们使用了大致以下公式推导出这些数值：

1. `TX_OVERHEAD_GAS = MAX_GAS_PER_BATCH / MAX_TXS_IN_BATCH`。对于 L1->L2 交易，我们使用了 `MAX_GAS_PER_BATCH = 80kk` 和 `MAX_TXS_IN_BATCH = 10k`。`MAX_GAS_PER_BATCH / MAX_TXS_IN_BATCH = 8k`，但我们决定使用 10k 的值，以更好地考虑 Operator 存储有关交易信息的负载。

2. `TX_MEMORY_OVERHEAD_GAS = MAX_GAS_PER_BATCH / MAX_MEMORY_FOR_BATCH`。对于 L1->L2 交易，我们使用了 `MAX_GAS_PER_BATCH = 80kk` 和 `MAX_MEMORY_FOR_BATCH = 32 * 600_000`。`MAX_GAS_PER_BATCH / MAX_MEMORY_FOR_BATCH = 4`，但我们决定使用 10 gas 的值，以更好地考虑 Operator 存储有关交易信息的负载。

未来的工作将集中在完全删除交易插槽数量的限制以及增加内存限制。

### Note on L1->L2 transactions

上述公式适用于 L1->L2 交易。然而，请注意 `gas_per_pubdata` 仍然保持恒定为 `800`。这意味着对于 L1->L2 交易，可以使用更高的 `baseFee`，以确保 `gas_per_pubdata` 在不考虑 pubdata 价格的情况下仍保持在该值。

### Refunds

请注意，用于费用模型的常量是概率性的，也就是说，我们无法提前确切知道一个批次为何会被封存。这些常量旨在覆盖 Operator 在较长时间内的费用，因此我们不会退还因交易带来的批次封存开销，因为这些资金是用于覆盖那些未全额支付批次资源费用的交易。

### Refunds for repeated writes

ZKsync Era 是基于状态差异的 rollup，也就是说，pubdata 是为存储变化而发布的，而不是为了交易。这意味着每当用户写入存储插槽时，就会产生一定数量的 pubdata。然而，并非所有写入都是相同的：

- 如果一个插槽已经在之前的某个批次中被写入，该插槽会收到一个短 ID，从而在状态差异中需要更少的 pubdata。
- 根据写入插槽的 `value`，可以使用各种压缩优化，这也应反映出来。
- 也许该插槽已经在此批次中被写入，因此我们不需要为其收取任何费用。

关键部分在于这种退款是内联的（即，与开销退款不同，它们在执行期间发生，而不是在整个交易处理完后发生），并且由 Operator 强制执行。目前，Operator 是决定提供何种退款的人。

## How ZKsync Era charges for pubdata

ZKsync Era 是一个基于状态差异的 rollup。这意味着在交易执行前无法确切知道交易将消耗多少 pubdata。我们可以通过以下方式对 pubdata 进行收费：每当用户执行会生成 pubdata 的操作（如写入存储、发布 L2->L1 消息等）时，我们直接从执行上下文中收取 `pubdata_bytes_published * gas_per_pubdata` 的费用。

然而，这种方法存在以下缺点：

- 这将使执行与 EVM 的差异很大。
- 它容易导致不必要的开销。例如，在重入锁的情况下，用户仍然需要支付初始价格来标记锁的使用。最终价格会被退还，但它仍然会影响用户体验。
- 如果我们希望对交易可能使用的计算量施加某种限制（称之为 `MAX_TX_GAS_LIMIT`），这意味着在一次交易中，发布的 pubdata 不能超过 `MAX_TX_GAS_LIMIT / gas_per_pubdata`，这使得该限制要么太小，要么迫使我们提高 `baseFee` 以防止数字增长过快。

为避免上述问题，我们需要在 pubdata 消耗的 gas 和执行中消耗的 gas 之间进行某种方式的解耦。尽管基于 calldata 的 rollup 会对 calldata 进行预收费，但我们无法这样做，因为确切的状态差异只有在交易完成后才知道。我们将采用后收费的方式。基本上，我们会保持一个计数器，跟踪已使用的 pubdata 量，并在交易结束时向用户收取 calldata 的费用。

后收费的问题在于，用户可能会在交易中用尽所有的 gas，这样我们将无法为 pubdata 收费。不过，请注意，如果交易被回滚，与其相关的状态更改也会被回滚。因此，每当我们需要向用户收费 pubdata 时，但其 gas 不足，交易将被回滚。用户将支付计算费用，但不会产生状态变化（因此也不会产生 pubdata）。

所以它的工作方式如下：

1. 首先，我们固定到目前为止发布的 pubdata 数量。将其表示为 `basePubdataSpent`。
2. 我们执行交易的验证。
3. 我们检查 `(getPubdataSpent() - basePubdataSpent) * gasPerPubdata <= gasLeftAfterValidation` 是否成立。如果不成立，则交易没有足够的资金来覆盖其自身的费用，因此应该被拒绝（不同于回滚，这意味着交易甚至不会被包含在区块中）。
4. 我们执行交易本身。
5. 我们再次进行步骤 (3) 的检查，但现在如果交易没有足够的 gas 来支付 pubdata，它将被回滚，即用户仍然支付计算费用来覆盖其交易，但状态将回滚。
6. （可选，在使用 paymaster 的情况下）我们重复步骤 (4-5)，但现在是为 paymaster 的 `postTransaction` 方法执行。

在内部级别，pubdata 计数器以以下方式进行修改：

- 当存在存储写入时，操作员需要提供要增加多少 pubdata 计数器的值。请注意，如果此值为负值（例如，在使用重入防护时，存储差异被回滚），则此值可以为负数。目前对操作员可以对 pubdata 收费的数量没有限制。
- 每当需要将字节块发布到 L1 时（例如，发布字节码），负责的系统合约将通过 `bytes_to_publish` 来增加 pubdata 计数器。
- 每当帧中有回滚时，pubdata 计数器也会被回滚，类似于存储和事件。

后收费的方式消除了不必要的开销，并将执行中用于数据可用性的 gas 与用于执行的 gas 分离，这消除了对 `gasPerPubdata` 的任何限制。

## Example for a queue of withdrawals

```solidity

struct Withdrawal {
   address token;
   address to;
   uint256 amount;
}

Withdrawals[] queue;
uint256 lastProcessed;

function processNWithdrawals(uint256 N) external nonReentrant {
  uint256 current = lastProcessed + 1;
  uint256 lastToProcess = current + N - 1;

  while(current <= lastToProcess) {
    // If the user provided some bad token that takes more than MAX_WITHDRAWAL_GAS
    // to transfer, it is the problem of the user and it will stall the queue, so
    // the `_success` value is ignored.
    Withdrawal storage currentQueue = queue[current];
    (bool _success, ) = currentQueue.token.call{gas: MAX_WITHDRAWAL_GAS}(abi.encodeWithSignature("transfer(to,amount)", currentQueue.to, currentQueue.amount));
    current += 1;
  }
  lastProcessed = lastToProcess;
}
```

上面的合约支持一个 withdrawals 队列。这个队列支持任何类型的 token，包括潜在的恶意 token。然而，该队列永远不会卡住，因为`MAX_WITHDRAWAL_GAS`确保即使恶意 token 进行大量计算，它也会受到这个数字的限制，因此调用`processNWithdrawals`的用户不会花费超过每个 token 的`MAX_WITHDRAWAL_GAS`。

上述假设在预收费模型（基于 calldata 的 rollups）或即用即付模型（1.5.0 版本之前的 Era）中有效。然而，在后收费模型中，`MAX_WITHDRAWAL_GAS`限制了在交易中可以完成的计算量，但它并不限制可以发布的 pubdata 数量。因此，如果这样的功能发布了非常大的 L1->L2 消息，可能会导致整个顶层交易失败。这意味着这样的队列可能会被阻塞。

### Limiting the `gas_per_pubdata`

如前所述，ZKsync 上的交易取决于 L1 gas 成本的波动，以发布批次的 pubdata，验证 proofs 等。出于这个原因，特定于 ZKsync 的 EIP712 交易包含了`gas_per_pubdata_limit`字段，表示操作员可以向用户收取的每个 pubdata 字节的最大`gas_per_pubdata`。

对于 Ethereum 交易（不包含此字段），使用区块的`gas_per_pubdata`。

### Improvements in the upcoming releases

上面解释的费用模型，虽然功能齐全，但存在一些已知问题。这些问题将在接下来的升级中解决。

### L1->L2 transactions do not pay for their execution on L1

`executeBatches`操作在 L1 上执行时的复杂度为`O(N)`，其中`N`是我们在批次中拥有的优先操作数量。每个执行的优先操作都会被弹出，因此会产生存储修改的成本。目前，我们不对此收费。

### ZKsync Era Fee Components (Revenue & Costs)

1. On-Chain L1 Costs

    - **L1 Commit Batches**: 提交批次交易将 pubdata（即更新的存储插槽列表）提交到 L1。提交交易的成本计算公式为 `constant overhead + price of pubdata`。`constant overhead`成本在 L1 提交交易中的 L2 交易之间平均分配，但仅在更高的交易负载下分配。至于`price of pubdata`，我们已知每个 L2 交易消耗了多少 pubdata，因此会直接向其收费。多个 L1 批次可以包含在单个提交交易中。

    - **L1 Prove Batches**: 一旦生成了链下的证明，就会提交到 L1，使得 rollup 批次最终确定。目前，每个证明仅包含一个 L1 批次。

    - **L1 Execute Batches**: 执行批次交易处理 L2->L1 消息，并将执行的优先操作标记为已处理。多个 L1 批次可以包含在单个执行交易中。

    - **L1 Finalize Withdrawals**: 虽然不严格属于 L1 费用的一部分，但 L2->L1 提现的最终处理费用由 Matter Labs 覆盖。最终处理提现交易处理从 ZKsync Era 到 Ethereum 的用户 token 提现。多个 L2 提现交易包含在每个最终提现交易中。

2. On-Chain L2 Revenue

    - **L2 Transaction Fee**: 这是用户在 ZKsync Era 上完成交易时支付的费用。计算公式为：

    `gasLimit x baseFeePerGas - refundedGas x baseFeePerGas`

    或者更简单地表示为：

    `gasUsed x baseFeePerGas`

3. Profit = L2 Revenue - L1 Costs - Off-Chain Infrastructure Costs

