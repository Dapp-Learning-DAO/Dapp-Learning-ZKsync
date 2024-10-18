# Ethereum Differences

## EVM Instructions

### CREATE, CREATE2

在 ZKsync Era 上，合约部署是通过使用字节码的哈希来执行的，而 EIP712 交易的 `factoryDeps` 字段包含了字节码。实际的部署通过将合约的哈希提供给 `ContractDeployer` 系统合约来完成。

为了保证 `create` / `create2` 函数的正确操作，编译器必须提前知道已部署合约的字节码。编译器将 calldata 参数解释为 `ContractDeployer` 的不完整输入，因为其余部分由编译器在内部填充。Yul 的 `datasize` 和 `dataoffset` 指令已调整为返回常量大小和字节码哈希，而不是字节码本身。

以下代码应按预期工作：

```solidity
MyContract a = new MyContract();
MyContract a = new MyContract{salt: ...}();
```

下列代码也应该可以工作，但必须显式测试以确保其预期功能：

```solidity
bytes memory bytecode = type(MyContract).creationCode;
assembly {
    addr := create2(0, add(bytecode, 32), mload(bytecode), salt)
}
```

以下代码将无法正确运行，因为编译器事先不知道字节码：

```solidity
function myFactory(bytes memory bytecode) public {
   assembly {
      addr := create(0, add(bytecode, 0x20), mload(bytecode))
   }
}
```

不幸的是，在编译时无法区分上述情况。因此，我们强烈建议为任何使用 `type(T).creationCode` 部署子合约的工厂包含测试。

由于在 ZKsync Era 上部署代码和运行时代码被合并在一起，我们不支持 `type(T).runtimeCode`，并且它始终会产生编译时错误。

### Address derivation

对于 zkEVM 字节码，ZKsync Era 使用了一种与 Ethereum 不同的地址推导方法。精确的公式可以在我们的 SDK 中找到，如下所示：

```solidity
export function create2Address(sender: Address, bytecodeHash: BytesLike, salt: BytesLike, input: BytesLike) {
  const prefix = ethers.utils.keccak256(ethers.utils.toUtf8Bytes("zksyncCreate2"));
  const inputHash = ethers.utils.keccak256(input);
  const addressBytes = ethers.utils.keccak256(ethers.utils.concat([prefix, ethers.utils.zeroPad(sender, 32), salt, bytecodeHash, inputHash])).slice(26);
  return ethers.utils.getAddress(addressBytes);
}

export function createAddress(sender: Address, senderNonce: BigNumberish) {
  const prefix = ethers.utils.keccak256(ethers.utils.toUtf8Bytes("zksyncCreate"));
  const addressBytes = ethers.utils
    .keccak256(ethers.utils.concat([prefix, ethers.utils.zeroPad(sender, 32), ethers.utils.zeroPad(ethers.utils.hexlify(senderNonce), 32)]))
    .slice(26);

  return ethers.utils.getAddress(addressBytes);
}
```

由于字节码与 Ethereum 不同，因为 ZKsync 使用了修改版的 EVM，从字节码哈希推导的地址也会有所不同。这意味着在 Ethereum 和 ZKsync 上部署的相同字节码将具有不同的地址，并且 Ethereum 地址在 ZKsync 上仍然是可用且未使用的。如果当 zkEVM 达到与 EVM 的一致性时，地址推导将会更新以匹配 Ethereum，并且相同的字节码将在两个链上具有相同的地址，部署到不同地址的字节码在 ZKsync 上可能会部署到相同的与 Ethereum 匹配的地址上。

### CALL, STATICCALL, DELEGATECALL

在 EVM 中，如果 outsize（输出数据的大小）不为 0，即使返回数据的大小比预期的小，EVM 也会先将内存分配为 out + outsize（向上取整，以 `word` 为单位），然后再写入返回数据。
而 zkSync 的处理方式与之不同, 返回数据的复制和写入是在调用完成之后才进行的，而不是像 EVM 那样提前分配内存。因此，zkSync 需要通过迭代返回数据的循环，进行附加检查。如果 `out + outsize > returndataSize`（返回的数据大小），则会触发 panic。
这意味着 zkSync 更倾向于在操作结束后再去分配内存并复制返回数据。zkSync 不会像 EVM 那样，因提前内存分配而导致不必要的内存增长和 panic。

```solidity
success := call(gas(), target, 0, in, insize, out, outsize) // grows to 'min(returndatasize(), out + outsize)'
```

```solidity
success := call(gas(), target, 0, in, insize, out, 0) // memory untouched
returndatacopy(out, 0, returndatasize()) // grows to 'out + returndatasize()'
```

另外，在 ZKsync Era 上没有原生支持传递 Ether，因此它是由一个名为 `MsgValueSimulator` 的特殊系统合约处理的。模拟器接收被调用者地址和 Ether 数量，执行所有必要的余额变更，然后调用。

### MSTORE, MLOAD

与 EVM 不同，在 EVM 上内存的增长是以 `words` 为单位的，而在 zkEVM 上内存的增长是以`bytes` 为单位的。例如，如果你在 zkEVM 上写入 `mstore(100, 0)`，则 `msize` 将为 132 ，而在 EVM 上它将为 160 。需要注意的是，EVM 中内存支付的增长是二次的，而在 zkEVM 上费用是按 1 erg 每字节线性计算的。

另外，我们的编译器有时可以优化未使用的内存读取/写入。这可能会导致 `msize` 与 Ethereum 不同，因为分配的字节更少，导致 EVM 中 panic 的情况，而 zkEVM 不会因为内存增长的差异而出现这种情况。

### CALLDATALOAD, CALLDATACOPY

如果 `calldataload(offset)` 的偏移量大于 `2^32-33`，则执行将会 panic。

在 zkEVM 内部，`calldatacopy(to, offset, len)` 只是一个在每次迭代时使用 `calldataload` 和 `mstore` 的循环。这意味着如果 `2^32-32 + offset % 32 < offset + len`，代码将 panic。

### RETURN, STOP

构造函数返回不可变值。如果你在 zkSync Era 上的汇编块中使用 `RETURN` 或 `STOP`，它将留下未初始化的不可变变量。

```solidity
contract Example {
    uint immutable x;

    constructor() {
        x = 45;

        assembly {
            // The statements below are overridden by the zkEVM compiler to return
            // the array of immutables.

            // The statement below leaves the variable x uninitialized.
            // return(0, 32)

            // The statement below leaves the variable x uninitialized.
            // stop()
        }
    }

    function getData() external pure returns (string memory) {
        assembly {
            return(0, 32) // works as expected
        }
    }
}

```

### TIMESTAMP, NUMBER

有关 ZKsync Era 上的区块的更多信息，包括 `block.timestamp` 和 `block.number` 之间的区别，请查看 [blocks on ZKsync Documentation](https://zksync-docs.io)。

**Changes From the Previous Protocol Version**
对 ZKsync Era 上某些区块属性的实现进行了修改。有关执行的更改的详细信息，请访问 [GitHub 上的公告](https://github.com/announcement)。

### COINBASE

返回 Bootloader 合约的地址，在 ZKsync Era 上为 `0x8001`。

### DIFFICULTY, PREVRANDAO

在 ZKsync Era 上返回一个恒定的值 `2500000000000000`。

### BASEFEE

在 ZKsync Era 上，这不是一个常量，而是由费用模型定义的。大多数情况下，它是 0.25 gwei，但在非常高的 L1 gas 价格下可能会上升。

### SELFDESTRUCT

被认为有害，并在 [EIP-6049](https://eips.ethereum.org/EIPS/eip-6049) 中被弃用。

使用 zkEVM 编译器时总是会产生编译时错误。

### CALLCODE

在 [EIP-2488](https://eips.ethereum.org/EIPS/eip-2488) 中被弃用，建议使用 `DELEGATECALL`。

使用 zkEVM 编译器时总是会产生编译时错误。

### PC

在 Yul 和 Solidity 版本 `>=0.7.0` 中不可访问，但在 Solidity `0.6` 中可访问。

### CODESIZE

| Deploy code                          | Runtime code  |
| ------------------------------------ | ------------- |
| Size of the constructor arguments    | Contract size |

Yul 使用了一条特殊的指令 `datasize` 来区分合约代码和构造函数的参数，因此我们在 ZKsync Era 的部署代码中将 `datasize` 替换为 `0`，将 `codesize` 替换为 `calldatasize`。这样，当 Yul 计算 `calldata` 大小时，`sub(codesize, datasize)` 的结果就是构造函数参数的大小。

```solidity
contract Example {
    uint256 public deployTimeCodeSize;
    uint256 public runTimeCodeSize;

    constructor() {
        assembly {
            deployTimeCodeSize := codesize() // return the size of the constructor arguments
        }
    }

    function getRunTimeCodeSize() external {
        assembly {
            runTimeCodeSize := codesize() // works as expected
        }
    }
}
```

### CODECOPY

```solidity
contract Example {
    constructor() {
        assembly {
            codecopy(0, 0, 32) // behaves as CALLDATACOPY
        }
    }

    function getRunTimeCodeSegment() external {
        assembly {
            // Behaves as 'memzero' if the compiler is run with the old (EVM assembly) codegen,
            // since it is how solc performs this operation there. On the new (Yul) codegen
            // `CALLDATACOPY(dest, calldatasize(), 32)` would be generated by solc instead, and
            // `CODECOPY` is safe to prohibit in runtime code.
            // Produces a compile-time error on the new codegen, as it is not required anywhere else,
            // so it is safe to assume that the user wants to read the contract bytecode which is not
            // available on zkEVM.
            codecopy(0, 0, 32)
        }
    }
}
```

### EXTCODECOPY

在 zkEVM 架构上无法访问合约 bytecode。它的大小只能通过 `CODESIZE` 和 `EXTCODESIZE` 访问。

`EXTCODECOPY` 在 zkEVM 编译器中总是会产生编译时错误。

### DATASIZE, DATAOFFSET, DATACOPY

合约部署由 zkEVM 协议的两个部分处理：编译器前端和一个名为 `ContractDeployer` 的系统合约。

在编译器前端，部署合约的代码会被替换为其哈希值。这个哈希值由 `dataoffset` Yul 指令或 `PUSH [$]` EVM 传统汇编指令返回。然后，哈希值会传递给 `datacopy` Yul 指令或 `CODECOPY` EVM 传统指令，这些指令会将哈希值写入 `ContractDeployer` 调用的 `calldata` 的正确位置。

`ContractDeployer` 调用数据由以下几个元素组成：

| 元素 | 偏移量 | 大小 |
| ---- | ------ | ---- |
| Deployer method signature | 0 | 4 |
| Salt | 4 | 32 |
| Contract hash | 36 | 32 |
| Constructor calldata offset | 68 | 32 |
| Constructor calldata length | 100 | 32 |
| Constructor calldata | 132 | N |

这些数据可以逻辑上分为头部（前 132 字节）和构造函数 `calldata`（其余部分）。

头部取代了 EVM 管道中的合约代码，而构造函数 `calldata` 保持不变。因此，`datasize` 和 `PUSH [$]` 返回头部大小（132 字节），并且由 `solc` 在其上分配构造函数参数的空间。

最后，`CREATE` 或 `CREATE2` 指令将 132 + N 字节传递给 `ContractDeployer` 合约，这会进行所有必要的状态更改，并在发生错误时返回合约地址或零。

如果传递了一些 Ether，那么对 `ContractDeployer` 的调用也将像普通调用一样通过 `MsgValueSimulator`。

我们不建议将 `CREATE` 用于除了使用 `new` 操作符创建合约之外的任何用途。然而，许多合约在汇编块中创建合约，因此必须确保行为与上面描述的逻辑兼容。

Yul example:

```solidity
let _1 := 128                                       // the deployer calldata offset
let _2 := datasize("Callable_50")                   // returns the header size (132)
let _3 := add(_1, _2)                               // the constructor arguments begin offset
let _4 := add(_3, args_size)                        // the constructor arguments end offset
datacopy(_1, dataoffset("Callable_50"), _2)         // dataoffset returns the contract hash, which is written according to the offset in the 1st argument
let address_or_zero := create(0, _1, sub(_4, _1))   // the header and constructor arguments are passed to the ContractDeployer system contract
```

EVM legacy assembly example:

```plaintext
010     PUSH #[$]       tests/solidity/complex/create/create/callable.sol:Callable      // returns the header size (132), equivalent to Yul's datasize
011     DUP1
012     PUSH [$]        tests/solidity/complex/create/create/callable.sol:Callable      // returns the contract hash, equivalent to Yul's dataoffset
013     DUP4
014     CODECOPY        // CODECOPY statically detects the special arguments above and behaves like the Yul's datacopy
...
146     CREATE          // accepts the same data as in the Yul example above
```

## Nonce

在Ethereum中，每个账户都关联着一个称为nonce的唯一标识符。对于外部拥有的账户（EOAs），nonce实现了三个主要功能：防止网络上的重放攻击、确保交易按照预期的顺序执行，并作为在推导地址公式中使用的唯一标识符。每次交易执行后，nonce都会递增。

在智能合约的上下文中，nonce有一个独特的用途：它决定了从另一个合约部署的新合约的地址。当使用 `create` 或 `create2` 函数创建新合约时，nonce会递增，以表示部署了一个新合约。与EOAs不同，EOAs每次交易只能增加一次nonce，而智能合约则能够在一次交易中多次增加其nonce。

但是 ZKsync 有原生抽象账户，nonce 既要保护账户免于重放攻击，也要参与生成部署合约的地址生成。（考虑到ZKsync中的账户可以是智能合约，它们可以在一笔交易中部署多个合约。）

为了在交易验证和合约部署上下文中保持nonce的预期使用和方便性，ZKsync引入了两种不同的nonce：

- 交易nonce
- 部署nonce

交易nonce用于交易验证，而部署nonce则在发生合约部署时递增。通过这种方式，账户可以仅通过跟踪一个nonce值发送多笔交易，并且合约可以部署许多其他合约而不会与交易nonce发生冲突。

在ZKsync和Ethereum的nonce管理之间还有其他一些小的差异：

- 新创建的合约以 `deployment nonce` 值为零开始。这与Ethereum形成对比，在Ethereum中，遵循 EIP-161 的规定，新创建合约的nonce从1开始。

- 在ZKsync上，部署nonce仅在部署成功时递增。在Ethereum中，部署nonce在部署时更新，即使合约创建失败也是如此。

## Libraries

ZKsync 依赖于 `solc` 优化器来进行库的内联，因此只有当库已经被优化器内联时，才可以在不部署的情况下使用它。

已部署库的地址必须在项目配置中设置。这些地址会替换它们在IRs中的占位符：Yul中的 `linkersymbol` 和 EVM遗留汇编中的 `PUSHLIB`。

所有链接都发生在编译时。部署时链接不被支持。

## Precompiles

某些 EVM 加密预编译（特别是配对和 RSA）当前不可用。然而，优先支持配对功能，以允许在无需修改的情况下部署 ZK 链和像 Aztec/Dark Forest 这样的协议。

以太坊的加密原语（如 `ecrecover`、`keccak256`、`sha256`、`ecadd` 和 `ecmul`）被支持为预编译函数。您不需要进行任何操作，因为对这些预编译函数的调用全部由编译器在后台完成。

需要注意的是，这些预编译函数在通过 `delegatecall` 调用时，其 gas 成本和行为可能与以太坊上的不同。

## Native AA vs EIP 4337

ZKsync 的原生账户抽象与以太坊的 EIP 4337 都旨在增强账户的灵活性和用户体验，但它们在以下关键方面有所不同：

1. **Implementation Level**: ZKsync 的账户抽象集成在协议层级；然而，EIP 4337 避免了在协议层级的实现。

2. **Account Types**: 在 ZKsync Era 中，智能合约账户和 Paymaster 是一级公民。在底层，所有账户（即使是 EOAs）都表现得像智能合约账户；所有账户都支持 Paymaster 。

3. **Transaction Processing**: EIP 4337 为智能合约账户引入了一个独立的交易流程，该流程依赖于用户操作的独立 mempool 以及打包器节点，这些节点会将用户操作打包并发送到 EntryPoint 合约进行处理，导致了两个独立的交易流程。相比之下，在 ZKsync Era 中，EOA 和智能合约账户的交易共享一个统一的 mempool。在 ZKsync Era 中， Operator 负责打包交易，无论账户类型如何，并将其发送到 Bootloader（类似于 EntryPoint 合约），这导致只有一个 mempool 和交易流程。

4. **Paymasters support**: ZKsync Era 允许 EOA 和智能合约账户都能从 Paymaster 中受益，因为它只有一个交易流程。另一方面，EIP 4337 不支持 EOA 的 Paymaster ，因为 Paymaster 仅在智能合约账户的新交易流程中实现。

## Contract Deployment

### Overview of the differences in contract deployment

为了保持与 L1 相同的安全级别，ZKsync  Operator 需要在以太坊链上发布它所部署的每个合约的代码。但是，如果多个合约使用相同的代码进行部署， Operator 只需要在以太坊上发布一次。虽然合约的初始部署可能相对昂贵，但利用合约工厂多次部署相同代码的合约与 L1 相比可以节省大量成本。

这些特定要求确保 zkEVM 上智能合约的部署过程符合一个关键规则： Operator 必须在部署前知晓合约的代码。因此，部署合约只能通过 EIP712 交易来完成，其中包含了提供的字节码的 `factory_deps` 字段。

### Ethereum / ZKsync differences in contract deployment

#### How deploying contracts works on Ethereum

在以太坊上部署合约时，用户发送一笔交易到零地址 (`0x000...000`)，交易的 `data` 字段等于合约字节码与构造函数参数的连接。

#### How deploying contracts works on ZKsync

在 ZKsync Era 上部署合约时，用户调用 `ContractDeployer` 系统合约的 `create` 函数，提供要发布的合约哈希以及构造函数参数。合约字节码本身在交易的 `factory_deps` 字段中提供（因为它是 EIP712 交易）。如果合约是一个工厂（即它可以部署其他合约），这些合约的字节码也应包括在 `factory_deps` 中。

我们推荐使用 `hardhat-zksync-deploy` 插件，以简化部署过程。它提供了类和方法来处理所有部署要求，如生成合约字节码哈希等。

### Note on `factory_deps`

你可能会想，验证者如何获得执行代码所需的字节码哈希的 原象(preimage)。这就是工厂依赖项（简称为 `factory_deps`）概念的作用所在。工厂依赖项指的是一系列字节码哈希的列表，这些哈希对应的 原象(preimage)之前已在 L1 上公开（数据总是可用的）。

在底层，ZKsync 并不在其状态树中存储合约的字节码，而是存储经过特殊格式化的字节码哈希。你可以看到 `ContractDeployer` 系统合约接受的是已部署合约的字节码哈希，而不是其字节码。然而，为了使合约部署成功， Operator 需要知道字节码。交易的 `factory_deps` 字段正是为此而存在：它包含了 Operator 为使交易成功而需要知道的字节码。这些字节码在交易成功后发布到 L1 上，并被 Operator 永久视为“已知”。

一些使用示例包括：

- 一个明显的使用场景是当你部署一个合约时，你需要在 `factory_deps` 字段中提供其代码。
- 在 ZKsync 上，工厂（即可以部署其他合约的合约）不会存储其依赖项的字节码，而是仅存储它们的哈希。这就是为什么你需要在 `factory_deps` 字段中包含所有依赖项的字节码。

以上两个示例已经通过我们的 [hardhat-zksync-deploy](#) 无缝地在底层实现。

注意，工厂依赖项并不一定要被交易实际使用。这些只是标记，表示这些字节码应该通过这笔交易发布到 L1。如果你的合约包含了许多不同的工厂依赖项，而它们无法放入单个 L1 区块中，你可以将工厂依赖项的列表拆分到多个交易中。

例如，假设你想要部署合约 `A`，它还可以部署合约 `B` 和 `C`。这意味着你将有三个工厂依赖项用于你的部署交易：`A`，`B` 和 `C`。如果发布所有这些所需的 pubdata 太大而无法放入一个区块，你可以发送一笔仅包含工厂依赖项 `A` 和 `B` 的虚拟交易（假设它们的组合长度足够小），然后使用第二笔交易实际部署，并在其中提供合约 `C` 的字节码作为工厂依赖项。注意，如果某个合约本身大于每个区块的允许限制，则必须将其拆分成较小的部分。

### Contract size limit and format of bytecode hash

每个 zkEVM 字节码必须符合以下格式：

- 它的长度必须是32的倍数。
- 它的长度以字（32字节块）计量时应为奇数。换句话说，`bytecodeLength % 64 == 32`。
- 存在一个虚拟机限制，字节码不能超过 `2^16` 个32字节的字，即 `2^21` 字节。
- 引导加载程序有一个450999字节的内存限制用于提供pubdata，因此也限制了合约的大小。这个限制适用于不需要将字节码发布到基础层的 Validium ZK 链。
- 对于必须将已部署的字节码发布到基础层（例如，以太坊）的Rollup链，还有一个额外的pubdata限制，通常更小。默认情况下，对于使用 calldata DA 的ZK链，每个批次的此限制设置为100000字节，对于使用EIP-4844 blobs的ZK链，则为120000乘以`blobs`的数量。

ZKsync合约的32字节字节码哈希计算方式如下：

- 前2个字节表示字节码哈希格式的版本，目前等于 `[1,0]`。
- 第二个2字节表示字节码的长度，单位为32字节的字。
- 其余的28字节（即28个低位big-endian字节）等于合约字节码的`sha256`哈希的最后28字节。

### Smart contract security

智能合约安全性至关重要。智能合约中的单个漏洞可能导致资金损失。确保您的合约能够防范常见威胁。

一个常见的Solidity智能合约攻击是重入攻击。这种威胁利用合约代码中的漏洞，使攻击者能够反复调用提取资金的函数。

审计智能合约以发现安全漏洞可以防止盗窃和其他恶意活动。审计需要对合约代码及其底层逻辑进行彻底的审查，以识别可能被攻击者利用的任何漏洞或弱点。审计人员会寻找诸如缓冲区溢出、整数溢出等问题，这些安全问题可能导致资产丢失或其他不良后果。此审查过程应包括手动和自动测试，以确保所有漏洞都能被识别出来。

智能合约的审计过程应由具备必要知识和经验的专家进行，以识别潜在的安全风险。投资全面的审计可以帮助防止安全漏洞，保护投资者和用户免受损失、声誉损害和法律问题。因此，优先考虑智能合约安全并采取主动措施，确保在ZKSync Era网络上部署智能合约之前对其进行彻底的安全漏洞审计是非常重要的。

### Differences in `create()` behaviour

为了促进对 account abstraction 的支持，ZKsync 将每个账户的 `nonce` 分为两个部分：部署 `nonce` 和交易 `nonce`。部署 `nonce` 代表该账户使用 `create()` 操作码部署的合约数量，而交易 `nonce` 用于防止交易重放攻击。

这种区分意味着，尽管 ZKsync 上的 `nonce` 在智能合约中与以太坊类似，但对于外部拥有的账户（EOAs）来说，计算已部署合约的地址并不如以太坊上那么简单。

在以太坊上，可以通过公式 `hash(RLP[address, nonce])` 安全地确定合约地址。然而，在 ZKsync 上，建议等到合约部署完成后，再通过 `ContractDeployer` 发出的 `ContractDeployed` 事件捕获新部署合约的地址。SDK 会在后台处理所有这些过程，以简化工作流程。

要获得确定性的地址，您应该使用 `ContractDeployer` 中的 `create2` 方法。它也可以用于 EOAs。

### Deploying contracts from L1

在 ZKsync Era 上，也可以通过 L1-L2 通信来部署合约。

用于提交 L1->L2 交易的 interface 接受该交易所需的所有工厂依赖项列表。与它们一起工作的逻辑与默认 L2 部署相同。唯一的区别在于，由于用户已经在 L1 上发布了字节码的完整 原象(preimage)，因此不需要再次在 L1 上发布这些字节码。

