# Boojum 介绍（1）算术化与电路门

## 1. 证明系统与算术化

Boojum 的证明系统主要分为三部分：

1. 约束系统构建与算术化
2. Plonk PIOP
3. 基于 FRI 的多项式承诺
   
在算术化这一层，约束系统构建器提供了一个面向开发者的电路编写前端。开发者可以用 Rust 语言来开发电路。
构建器（CS Builder）在提供一些抽象接口的同时，也提供了一些实用的电路门，比如算术门（FMA Gate），查找门（Lookup Gate），u16, u32 按位对齐的整数，
布尔值，还有 Poseidon hash 门，这是一类常用的 zk-SNARK 友好的散列函数门。

Plonk PIOP 是最核心的交互式协议，证明算术化产生的一系列多项式之间满足特定的关系。

FRI 协议是一种 Low Degree Test 交互式协议，用来证明一个多项式的次数小于一个上界。基于 FRI，我们可以构造一个多项式承诺方案。FRI 相比 KZG10 承诺
方案的优势是，FRI 可以使用 Small Fields，比如 Goldilocks, BabyBear，M21 等等。Small Fields 更硬件友好，从而能显著提升 Prover 的计算性能。

Boojum 的算术化部分是理解 Boojum 证明系统入口。本文通过结合 Boojum 的源码，来介绍电路，证明系统中的基础概念，并给出一些简单的电路开发示例代码。

## 2. Boojum Plonkish 电路表格概述

Boojum 证明系统可以看成是 Plonk 证明系统的一个变种，因此支持 Plonkish 的电路约束表示，但是与经典的 Plonk 实现也有一些不同之处。

在经典 Plonk 证明系统中，一个表格是一个 $m\times n$ 的矩阵，其中每一行表示一条约束，总共有 $m$ 行。表格的每一列具有相同的用途和属性，
这是因为 Prover 会对表格的每一列进行多项式编码，并计算承诺。随后 Plonk 证明系统会采用交互式协议，来证明这些表格的各个列之间满足一些全局
的约束关系。

电路表格列分为两大部分：一部分为对 Verifier 不可见的 Witness 部分；另一部分为对 Verifier 公开可见的 Selector 和
Constant, 还有 Public-input 列。

选择器 Selector 列是电路表格中非常重要的概念。电路表格的每一行都是一条约束，但是可能是用的不同的逻辑来约束 `Witness Cell` 之间的关系。
假如整个电路表格有 $4$ 种不同的约束，那么就会有 $4$ 个 Selector，即每一行对应四个 Selector，但其中只有一个 Selector 的值为 `1`，其余为 `0`。
它的作用是指定当前行的约束逻辑到底是哪一种。举个例子，一个电路表格中有加法、乘法，还有查找约束，那么我们就需要至少三个 Selector 来区分电路表格中
的一行到底是加法运算还是乘法运算，或者是需要满足（附带的 Table）查找关系。

Boojum 的电路表格与经典的 Plonk 比较类似，但是也有不同的地方。主要体现在以下几个方面。

首先电路的 Selector 列会进行压缩表示。与经典 Plonk 不同的是，假如有 $k$ 个不同类型的约束，那么 Boojum 会仅采用不超过 $\log{k}$ 个 selector 来
指定某一行约束的类型。这样 Boojum 可以采用较少的 Selector 列来实现切换约束类型。但另一方面，较少的 Selector 会提升整个约束的 Degree，因此  Boojum 
采用了一个算法来根据约束的 Degree 分配 Selector 的数量，从而达到一个较好的平衡。

其次，电路会引入一部分名为 `Witness` 的列，与 `Variable` 列分离开。
但其实 `Witness` 这个命名容易与证明系统的概念 `Witness` 混淆。这里 `Witness` 的准确含义更接近 `Advice` 这个概念，指约束证明中的辅助值，并对 Verifier 不可见。
这部分表列和 `Variable` 列的最大区别是它不需要进行拷贝约束。这样做的优势是降低 Plonk 「约束等式多项式」的 Degree。因为不管在 Plonk 还是 Boojum 中，拷贝
约束都依赖了连乘证明，有多少需要拷贝约束的列，就需要有多少个多项式相乘，这会快速拉高「约束等式多项式」的 Degree。而分离开需要拷贝约束的列，可以缓解这个问题，尽可能
避免拉高「约束等式多项式」的 Degree 的各种因素。

Boojum 为了性能优化的考虑，尽量将同一种类型的门约束放置到表格的同一行，这样带来的好处是多个同一类型的约束可以复用相同的 `Constant` 列。同时，Boojum 约束系统构建器在
排布约束的时候，会优先合并那些相同类型的门。

此外，Boojum 还定义了一类被称为 Specialized Column 的列，区别于 General-purpose Column。这是为了考虑电路中可能会大量重复采用同一种门，那么我们可以把所有这种类型的门放在一起，
从而省去 Selector 列。


## 3. 最简电路代码示例

一个电路的构造和证明过程主要分为下面几个主要步骤

1. 设置电路配置参数。
2. 创建一个 Builder
3. 配置 Builder 需要用到的门
4. 创建 ConstraintSystem 的实例
5. 构建电路
6. 设置 FRI 证明参数
7. 产生证明

我们先看一个最简的电路构建和证明源码，看看上面的步骤是如何实现的。

```rust
fn simple_add() {

    type P = GoldilocksField;

    // 设置电路配置参数（几何结构）
    let geometry = CSGeometry {
        num_columns_under_copy_permutation: 4, // 拷贝约束的列宽度
        num_witness_columns: 0,                // advice 列宽度
        num_constant_columns: 2,               // 常数列宽度
        max_allowed_constraint_degree: 5,      // 每一行的约束 degree
    };

    let max_variables = 4; // variable 数量上限
    let max_trace_len = 4; // 电路表格的行数上限

    // 配置cs的函数
    fn configure<
        F: SmallField,
        T: CsBuilderImpl<F, T>,
        GC: GateConfigurationHolder<F>,
        TB: StaticToolboxHolder,
    >(
        builder: CsBuilder<T, F, GC, TB>,
    ) -> CsBuilder<T, F, impl GateConfigurationHolder<F>, impl StaticToolboxHolder> {

        let builder = ReductionGate::<_, 2>::configure_builder(
            builder,
            GatePlacementStrategy::UseGeneralPurposeColumns,
        );

        // 需要在cs中加入空操作门，用于 cs.pad_and_shrink()
        let builder =
            NopGate::configure_builder(builder, GatePlacementStrategy::UseGeneralPurposeColumns);

        builder
    }

    // cs builder: 约束系统的工厂类
    let builder_impl = CsReferenceImplementationBuilder::<GoldilocksField, P, DevCSConfig>::new(
        geometry,
        max_trace_len,
    );
    let builder = new_builder::<_, GoldilocksField>(builder_impl);

    let builder = configure(builder);
    let mut cs = builder.build(CircuitResolverOpts::new(max_variables));

    // 设置 x = 2
    let two = cs.alloc_single_variable_from_witness(GoldilocksField::TWO);

    // 2x = x + x
    let double_x = ReductionGate::reduce_terms(&mut cs, [GoldilocksField::ONE; 2], [two; 2]);

    // optional
    cs.pad_and_shrink();

    // 设置线程数量
    let worker = Worker::new_with_num_threads(1);
    let cs = cs.into_assembly::<Global>();

    // 配置 FRI 参数
    let lde_factor_to_use = 4;
    let proof_config = ProofConfig {
        fri_lde_factor: lde_factor_to_use,
        pow_bits: 0,
        merkle_tree_cap_size: 1,
        ..Default::default()
    };

    // 使用 `cs.prove_one_shot()` 同时生成 `proof` 和 `vk`
    // NOTE: 这是开发环境下的测试函数，在生产环境不建议这样使用
    let (proof, vk) = cs.prove_one_shot::<
        GoldilocksExt2,
        GoldilocksPoisedonTranscript,
        GoldilocksPoseidonSponge<AbsorptionModeOverwrite>,
        NoPow,
    >(&worker, proof_config, ());

    let builder_impl =
        CsVerifierBuilder::<GoldilocksField, GoldilocksExt2>::new_from_parameters(geometry);
    let builder = new_builder::<_, GoldilocksField>(builder_impl);

    let builder = configure(builder);
    let verifier = builder.build(());

    let is_valid = verifier.verify::<
        GoldilocksPoseidonSponge<AbsorptionModeOverwrite>,
        GoldilocksPoisedonTranscript,
        NoPow
    >(
        (),
        &vk,
        &proof,
    );

    // 检查证明是否验证通过
    assert!(is_valid);
}
```

### 3.1 电路几何结构。

代码中的 `geometry` 表示电路表格的结构，其中 `num_columns_under_copy_permutation` 表示有多少列放置变量。这些变量之间可以通过拷贝约束建立等价关系。
第二部分 `num_witness_columns` 表示有多少列被当作电路的辅助输入。针对上面的加法电路简单例子，并不需要这部分的表列，因此设置为 `0`。第三部分是常数列，
`num_constant_columns` 用来放置 ReductionGate 中的常数值。最后一个 `max_allowed_constraint_degree` 表示每行约束的多项式次数的上届。

最后 `max_variables` 用来指定电路中最大可使用的变量的个数，而 `max_trace_len` 指明电路表格的高度。

```rust
    let geometry = CSGeometry {
        num_columns_under_copy_permutation: 4, // 拷贝约束的列宽度
        num_witness_columns: 0,                // advice 列宽度
        num_constant_columns: 2,               // 常数列宽度
        max_allowed_constraint_degree: 5,      // 每一行的约束 degree
    };

    let max_variables = 4; // variable 数量上限
    let max_trace_len = 4; // 电路表格的行数上限
```

### 3.2 创建 Builder 

创建 Builder 需要使用 `CsReferenceImplementationBuilder` 构造一个 Builder 的一个具体实现，并输入电路的参数。
然后再调用 `CsBuilder` 模块中的 `new_builder()` 函数构造一个 Builder。

```rust
    // cs builder: 约束系统的工厂类
    let builder_impl = CsReferenceImplementationBuilder::<GoldilocksField, P, DevCSConfig>::new(
        geometry,
        max_trace_len,
    );
    let builder = new_builder::<_, GoldilocksField>(builder_impl);
```

### 3.3 配置 Builder

配置 Builder 的任务主要是需要在 Builder 中打开对各种基础门的支持开关，并对一些门指定参数。

```rust
    let builder = configure(builder);
```

下面是具体的配置函数的示例代码：

```rust
    // 配置cs的函数
    fn configure<
        F: SmallField,
        T: CsBuilderImpl<F, T>,
        GC: GateConfigurationHolder<F>,
        TB: StaticToolboxHolder,
    >(
        builder: CsBuilder<T, F, GC, TB>,
    ) -> CsBuilder<T, F, impl GateConfigurationHolder<F>, impl StaticToolboxHolder> {

        let builder = ReductionGate::<_, 2>::configure_builder(
            builder,
            GatePlacementStrategy::UseGeneralPurposeColumns,
        );

        // 需要在cs中加入空操作门，用于 cs.pad_and_shrink()
        let builder =
            NopGate::configure_builder(builder, GatePlacementStrategy::UseGeneralPurposeColumns);

        builder
    }
```

对于加法电路示例，我们只需要加入 `ReductionGate` 和 `NopGate` 即可。


### 3.4 创建一个 ConstraintSystem 实例

接下来的关键代码是利用 Builder 创建一个电路约束系统。这个可变变量 `cs` 是下面我们搭建电路反复传递的关键变量。

```rust
    let mut cs = builder.build(CircuitResolverOpts::new(max_variables));
```
### 3.5 利用基础门组件来构建电路

第一步是创建一个新的变量 `two`，第二步是创建两个常数 `(1, 1)`，然后和 `(two, two)` 输入到一个 `ReductionGate`
中，并将 `ReductionGate` 的输出创建为一个新的变量 `double_x`。

```rust
    // 设置 x = 2
    let two = cs.alloc_single_variable_from_witness(GoldilocksField::TWO);

    // 2x = x + x
    let double_x = ReductionGate::reduce_terms(&mut cs, [GoldilocksField::ONE; 2], [two; 2]);

    // optional
    cs.pad_and_shrink();
```

最后一步当电路搭建完毕后，需要调用 `cs.pad_and_shrink()` 把电路表格中剩余的行填充完。

### 3.6 设置 FRI 证明参数

在电路创建完毕之后，需要设置 FRI 的参数。尽管 FRI 协议是证明协议中的最后环节，但是设置 FRI 的参数会影响到
证明系统的初始化与多项式约束的产生参数，因此必须提前设置。

```rust
    // 设置线程数量
    let worker = Worker::new_with_num_threads(1);
    let cs = cs.into_assembly::<Global>();

    // 配置 FRI 参数
    let lde_factor_to_use = 4;
    let proof_config = ProofConfig {
        fri_lde_factor: lde_factor_to_use,
        pow_bits: 0,
        merkle_tree_cap_size: 1,
        ..Default::default()
    };
```

我们会在后续的文章中讨论这些 FRI 的含义以及 FRI 的原理，这里不再深入。

### 3.7 产生证明

我们使用 `cs.prove_one_shot()` 同时生成 `proof` 和 `vk`，
请注意这是开发环境下的测试函数，在生产环境不建议这样使用。

```rust

    let (proof, vk) = cs.prove_one_shot::<
        GoldilocksExt2,
        GoldilocksPoisedonTranscript,
        GoldilocksPoseidonSponge<AbsorptionModeOverwrite>,
        NoPow,
    >(&worker, proof_config, ());
```

产生的证明放入到变量 `proof` 中，然后可以用下面的 Verifier 代码检验。

```rust
    let is_valid = verifier.verify::<
        GoldilocksPoseidonSponge<AbsorptionModeOverwrite>,
        GoldilocksPoisedonTranscript,
        NoPow
    >(
        (),
        &vk,
        &proof,
    );

    // 检查证明是否验证通过
    assert!(is_valid);
```

## 4. 基本概念理解

本节，我们结合 Boojum 的源码，进一步分析 Boojum 电路中出现的众多概念及其基本原理。

### 4.1 什么是 Variable 

Boojum 中的值有以下三种类型，其中 `Variable` 和 `Witness` 是仅 Prover 可见，而`Constant` 是公开的。

变量 Variable 是 Boojum 电路中非常重要的概念。每一个 `Variable` 都有一个唯一的 Id, 它可以被「拷贝」到电路表格中的特定列的任意位置上。
起到串联各个电路 Gate 的作用。

他们的区别如下：

- Variables 仅 Prover 可见，可拷贝

- Witnesses 仅 Prover 可见，不可以拷贝，也可称为 Advice

- Constants 公开常量

下面是 `Variable` 的类型声明源码，我们可以从这里切入 Boojum 的实现源码。

```rust
// from src/cs/mod.rs

pub struct Variable(pub(crate) u64); // 带有一个唯一标识 index 

impl Variable {
    #[inline(always)]
    pub const fn placeholder() -> Self {
        Self(PLACEHOLDER_BIT_MASK)
    }

    #[inline(always)]
    pub const fn as_variable_index(&self) -> u32 {
        debug_assert!(!self.is_placeholder());

        let var_idx = self.0 & LOW_U48_MASK;

        var_idx as u32
    }

    #[inline(always)]
    pub const fn is_placeholder(&self) -> bool {
        self.0 & PLACEHOLDER_BIT_MASK != 0
    }

    #[inline(always)]
    pub const fn from_variable_index(variable_index: u64) -> Self {
        Self(variable_index)
    }
}
```

有一个特殊的 Variable，`placeholder`，它仅仅表示一个占位符，不能用来链接 Gate。

全局类 `CS` 保存了一个 `next_available_place_idx` 的变量，每次自增一，用来保证每次产生新的
Variable 时，使用一个唯一的 index。
 
### 4.2 Variable 赋值过程

请看 `boolean_allocator.rs` 中的这样一段代码：

```rust
pub fn alloc_boolean_from_witness<F: SmallField, CS: ConstraintSystem<F>>(
    cs: &mut CS,
    witness_value: bool,
) -> Variable {
    debug_assert!(cs.gate_is_allowed::<Self>());

    // 生成一个不含值的variable
    let new_var = cs.alloc_variable_without_value();
    // 生成boolean的赋值门
    let gate = Self {
        var_to_enforce: new_var,
    };

    // true 被转化为 F::ONE，false 被转化为 F::ZERO
    let value = if witness_value { F::ONE } else { F::ZERO };

    // 将 variable 赋值到cs中
    cs.set_values(&Place::from_variables([new_var]), [value]);
    // 将门注册到 cs
    gate.add_to_cs(cs);

    // 返回预期生成的新 variable
    new_var
}
```

可以看到，设置一个 `variable`的步骤如下：

1. 首先生成一个不含值的 `variable`，使用 `let new_var = cs.alloc_variable_without_value();`

2. 然后，使用 `cs.set_values(&Place::from_variables([new_var]), [value]);`，`cs` 对象会把 `value` 设置到 `variables_storage` 内。

再进一步观察 `set_values` 的代码：

```rust
#[inline]
fn set_values<const N: usize>(&mut self, places: &[Place; N], values: [F; N]) {
    if CFG::WitnessConfig::EVALUATE_WITNESS == true {
        places.iter().zip(values).for_each(|(a, b)| {

            // 此处，value被赋值到variables_storage中
            self.variables_storage.get_mut().unwrap().set_value(*a, b);
        });
    }
}
```

3. 最后通过 `gate.add_to_cs(&cs)` 把 `gate` 注册到约束系统中

```rust
pub fn add_to_cs<F: SmallField, CS: ConstraintSystem<F>>(self, cs: &mut CS) {
    // 门必须被设置为允许
    debug_assert!(cs.gate_is_allowed::<Self>());

    if <CS::Config as CSConfig>::SetupConfig::KEEP_SETUP == false {
        return;
    }

    // 对于general column和specialized column有不同的place方案
    match cs.get_gate_placement_strategy::<Self>() {
        GatePlacementStrategy::UseGeneralPurposeColumns => {
            // 获取下一个可以放置新gate的行
            let offered_row_idx = cs.next_available_row();
            // 获取该行的最大长度
            let capacity_per_row = self.capacity_per_row(&*cs);
            // tooling是一个二元组，存放的是行列坐标
            let tooling: &mut NextGateCounterWithoutParams = cs
                .get_gates_config_mut()
                .get_aux_data_mut::<Self, _>()
                .expect("gate must be allowed");
            let (row, num_instances_already_placed) =
                find_next_gate_without_params(tooling, capacity_per_row, offered_row_idx); 
                // 已经存在这样的门且门所在的行未满，则row != offered_row_idx，否则row == offered_row_idx
            drop(tooling);

            // now we can use methods of CS to inform it of low level operations
            let offset = num_instances_already_placed * PRINCIPAL_WIDTH;
            if offered_row_idx == row {
                // 当前是新行，则在当前行放置gate
                cs.place_gate(&self, row);
            }
            // 无论如何，都在当前行放入需要置换证明的variable
            cs.place_variable(self.variable_to_set, row, offset);
        }
        GatePlacementStrategy::UseSpecializedColumns {
            num_repetitions,
            share_constants: _,
        } => {
            // gate knows how to place itself
            let capacity_per_row = num_repetitions;
            let tooling: &mut NextGateCounterWithoutParams = cs
                .get_gates_config_mut()
                .get_aux_data_mut::<Self, _>()
                .expect("gate must be allowed");
            let (row, num_instances_already_placed) =
                find_next_specialized_gate_without_params(tooling, capacity_per_row);
                cs.place_gate_specialized(&self, num_instances_already_placed, row);
                cs.place_variable_specialized::<Self>(
                self.var_to_enforce,
                num_instances_already_placed,
                row,
                0,
            );
        }
    }
}
```

### 4.3 什么是 Place

`Place` 表示电路表格中的一个单元格。下面是它的定义：

```rust
pub struct Place(pub(crate) u64);

pub enum VariableType {
    CopyableVariable = 0,
    Witness = 1,
}
```

`Place` 是 `Variable` 和 `Witness` 的抽象，可以分别转化为 `Variable` 和 `Witness`，它的默认成员变量可以认为是唯一标记该 `Place` 的id。

```rust
impl Place {
    // 转化为variable
    #[inline(always)]
    pub const fn as_variable(&self) -> Variable {
        debug_assert!(self.is_copiable_variable());
        debug_assert!(!self.is_placeholder());

        Variable(self.0)
    }

    // 转化为witness
    #[inline(always)]
    pub const fn as_witness(&self) -> Witness {
        debug_assert!(self.is_witness());
        debug_assert!(!self.is_placeholder());

        let idx = self.0 & LOW_U48_MASK;

        Witness::from_witness_index(idx)
    }

    // 从variable转化而来
    #[inline(always)]
    pub const fn from_variable(variable: Variable) -> Self {
        debug_assert!(variable.is_placeholder() == false);
        Self(variable.0)
    }

    // 从witness转化而来
    #[inline(always)]
    pub const fn from_witness(witness: Witness) -> Self {
        Self(witness.0 | WITNESS_BIT_MASK)
    }
}
```

`Place`被频繁用在 `Variable` 和 `Witness` 的方法中，在下面的例子中可以看到。

### 4.4 什么是 Tooling


在每次在电路表格中摆放一个门（或者约束）时，我们尽量要把同样类型的门放在同一行内。除非当前行被放满了，需要另
开辟一个新行。那么约束系统需要维护一个全局的 HashMap，记录每一种门在电路表格中的摆放区域。这就需要用到 Tooling 
这个结构。

例如在上面的代码中，出现了下面的片段：

```rust
//...
    // 对于general column和specialized column有不同的place方案
    match cs.get_gate_placement_strategy::<Self>() {
        GatePlacementStrategy::UseGeneralPurposeColumns => {
            // 获取下一个可以放置新gate的行
            let offered_row_idx = cs.next_available_row();
            // 获取该行的最大长度
            let capacity_per_row = self.capacity_per_row(&*cs);
            // tooling是一个二元组，存放的是行列坐标，下文进行分析～
            let tooling: &mut NextGateCounterWithoutParams = cs
                .get_gates_config_mut()
                .get_aux_data_mut::<Self, _>()
                .expect("gate must be allowed");
            let (row, num_instances_already_placed) =
                find_next_gate_without_params(tooling, capacity_per_row, offered_row_idx); // 已经存在这样的门且门所在的行未满，则row != offered_row_idx，否则row == offered_row_idx
            drop(tooling);

            // now we can use methods of CS to inform it of low level operations
            let offset = num_instances_already_placed * PRINCIPAL_WIDTH;
            if offered_row_idx == row {
                // 当前是新行，则在当前行放置gate
                cs.place_gate(&self, row);
            }
            // 无论如何，都在当前行放入需要置换证明的variable
            cs.place_variable(self.variable_to_set, row, offset);
        }
//...
```

看下生成`tooling`的`get_aux_data_mut`函数。

```rust
fn get_aux_data_mut<G: Gate<F>, T: 'static + Send + Sync + Clone>(&mut self) -> Option<&mut T> {
    if TypeId::of::<GG>() == TypeId::of::<G>() {
        // 如果GG匹配了当前类型G
        debug_assert!(TypeId::of::<TT>() == TypeId::of::<T>());
        unsafe {
            // 返回当前类型的aux数据
            let casted: &mut T = &mut *(&mut self.0.aux as *mut TT).cast() as &mut T;

            Some(casted)
        }
    } else {
        // 遍历链表下一项
        self.1.get_aux_data_mut::<G, T>()
    }
}
```

如果在链表中找到了匹配的Gate类型，就会返回对应Gate的辅助数据。也就是说，每一个Gate都有单独对应的辅助数据。

再来看下使用 `tooling` 的 `find_next_specialized_gate_without_params` 函数。

```rust
#[inline]
pub(crate) fn find_next_gate_without_params(
    tooling: &mut Option<(usize, usize)>,
    capacity_per_row: usize,
    offered_row_idx: usize, // 下一个未使用的gate行
) -> (usize, usize) {
    debug_assert!(capacity_per_row >= 1);

    if capacity_per_row == 1 {
        // 如果只有一列，则直接返回
        return (offered_row_idx, 0);
    }

    if let Some((existing_row_idx, num_instances_already_placed)) = tooling.take() {
        if num_instances_already_placed == capacity_per_row {
            // capacity_per_row (128)
            // 行数到头了，另起一列
            // we need a new one
            *tooling = Some((offered_row_idx, 1));

            (offered_row_idx, 0)
        } else {
            *tooling = Some((existing_row_idx, num_instances_already_placed + 1));

            (existing_row_idx, num_instances_already_placed)
        }
    } else {
        // 如果之前没有使用过find_next_gate_without_params
        // we need a new one
        *tooling = Some((offered_row_idx, 1));

        (offered_row_idx, 0)
    }
}
```

`find_next_gate_without_params`的作用就是判断当前`gate`的放置位置（如果需要放置的话）和`variable`放置的列。
如果当前`gate`还没有被放置到cs中，就新起一行放置当前`gate`和对应`variable`，否则就查看已经放置当前`gate`的行。
如果这一行还未填满`variable`，则填入`gate`对应的`variable`，否则也另起一行。


每一个类型的 Gate 会附带一个 Tooling 的结构，保存 Gate 所正在分配的行空间。这么做的目的是把同样类型的 Gate 分配到同一行。
这样可以有效利用电路表格空间。

### 4.5 constant赋值过程

```rust
pub fn allocate_constant<CS: ConstraintSystem<F>>(cs: &mut CS, constant_to_add: F) -> Variable {
    // 必须允许ConstantsAllocatorGate
    debug_assert!(cs.gate_is_allowed::<Self>());

    // 首先查找是否已经赋值过constant_to_add
    // tooling存储了所有已存在的赋值
    let tooling: &mut ConstantToVariableMappingTool<F> = cs
        .get_static_toolbox_mut()
        .get_tool_mut::<ConstantToVariableMappingToolMarker, ConstantToVariableMappingTool<F>>()
        .expect("tool must be added");
    // 如果已经赋值过constant_to_add，则直接返回，不进行赋值
    if let Some(variable) = tooling.get(&constant_to_add).copied() {
        return variable;
    }

    // 废弃用过的tooling
    drop(tooling);

    // 设置一个未赋值的variable作为output
    let output_variable = cs.alloc_variable_without_value();

    // 把新的constant赋值到tooling
    let tooling: &mut ConstantToVariableMappingTool<F> = cs
        .get_static_toolbox_mut()
        .get_tool_mut::<ConstantToVariableMappingToolMarker, ConstantToVariableMappingTool<F>>()
        .expect("tool must be added");
    // existing值应为none
    let existing = tooling.insert(constant_to_add, output_variable);
    assert!(existing.is_none());
    drop(tooling);

    // 如果设置了EVALUATE_WITNESS (默认为1)
    // 将constant交给resolver，暂时还不知道resolver的作用
    if <CS::Config as CSConfig>::WitnessConfig::EVALUATE_WITNESS {
        let value_fn = move |_inputs: [F; 0]| [constant_to_add];

        let dependencies = [];

        cs.set_values_with_dependencies(
            &dependencies,
            &Place::from_variables([output_variable]),
            value_fn,
        );
    }

    // 如果设置了KEEP_SETUP (默认为1)
    // 生成一个常量门约束
    if <CS::Config as CSConfig>::SetupConfig::KEEP_SETUP == true {
        let gate = Self::new_to_enforce(output_variable, constant_to_add);
        gate.add_to_cs(cs);
    }

    output_variable
}
```

### 4.6 约束系统构建器：CSBuilder

约束系统构建器，是约束系统类的工厂类，我们可以简单理解为一个特定电路

CS（Constraint System 的缩写） 是我们真正使用的约束系统，而 CS builder 是 CS 的工厂类
CS builder 的作用是，在我们真正生成 CS 之前，定制 CS 上 `variables table` 的几何结构、LDE-degree（LDE 是 Low degree extension 的缩写） 
和门电路等属性生成 CS 之后，我们就无法更改 CS 的属性了

具体观察 CS builder 的相关源码：

```rust
pub trait CsBuilderImpl<F: SmallField, TImpl> {
    // Final 是生成 cs 的类型
    type Final<GC: GateConfigurationHolder<F>, TB: StaticToolboxHolder>;
    // CircuitResolverOpts 的类型
    type BuildParams<'a>;

    // 返回电路几何结构 geometry
    fn parameters<GC: GateConfigurationHolder<F>, TB: StaticToolboxHolder>(
        builder: &CsBuilder<TImpl, F, GC, TB>,
    ) -> CSGeometry;

    // 配置门电路
    fn allow_gate<
        GC: GateConfigurationHolder<F>,
        TB: StaticToolboxHolder,
        G: Gate<F>,
        TAux: 'static + Send + Sync + Clone,
    >(
        builder: CsBuilder<TImpl, F, GC, TB>,
        placement_strategy: GatePlacementStrategy,
        params: <<G as Gate<F>>::Evaluator as GateConstraintEvaluator<F>>::UniqueParameterizationParams,
        aux_data: TAux,
        // ) -> CsBuilder<TImpl, F, GC::DescendantHolder<G, TAux>, TB>;
    ) -> CsBuilder<TImpl, F, (GateTypeEntry<F, G, TAux>, GC), TB>;

    // tool 是自定义工具，比如可以是一个 hashmap（参考 ConstantsAllocatorGate）
    // 这些 tool 在 cs 中排成一个链表，通过 add_tool 可以将自定义 tool 插入链表
    fn add_tool<
        GC: GateConfigurationHolder<F>,
        TB: StaticToolboxHolder,
        M: 'static + Send + Sync + Clone,
        T: 'static + Send + Sync,
    >(
        builder: CsBuilder<TImpl, F, GC, TB>,
        tool: T,
        // ) -> CsBuilder<TImpl, F, GC, TB::DescendantHolder<M, T>>;
    ) -> CsBuilder<TImpl, F, GC, (Tool<M, T>, TB)>;

    // gates_configuration 存放的是 gate 对应的 placement strategy (general purpose column、specialized column)
    type GcWithLookup<GC: GateConfigurationHolder<F>>: GateConfigurationHolder<F>;

    // 配置 lookup table
    fn allow_lookup<GC: GateConfigurationHolder<F>, TB: StaticToolboxHolder>(
        builder: CsBuilder<TImpl, F, GC, TB>,
        lookup_parameters: LookupParameters,
    ) -> CsBuilder<TImpl, F, Self::GcWithLookup<GC>, TB>;

    // 构造 cs
    fn build<
        'a,
        GC: GateConfigurationHolder<F>,
        TB: StaticToolboxHolder,
        ARG: Into<Self::BuildParams<'a>>,
    >(
        builder: CsBuilder<TImpl, F, GC, TB>,
        params: ARG,
    ) -> Self::Final<GC, TB>;
}
```

### 4.7 电路门：Gate

gate 是门电路，boojum 中的 gate 支持自动计算输出、自动将中间结果写入电路的功能。
在 boojum 中我们无法直接在电路上赋值。为了实现多项式约束，我们只能通过更高级的 Gate 来操作电路
需要注意的是，在 boojum 中我们不需要关心每个 Gate 的 selector，系统会自动帮我们计算相应的 selector

Gate 的使用方法如下：
每个 Gate 都实现了 configure_builder 方法。在 cs builder 阶段，configure_builder 能将我们需要的门电路注册到 cs 中。
具体观察 ConstantsAllocatorGate 的 configure_builder：

```rust 
pub fn configure_builder<
    // placement strategy
    GC: GateConfigurationHolder<F>,
    // tool 类型
    TB: StaticToolboxHolder,
    // cs builder 类型
    TImpl: CsBuilderImpl<F, TImpl>,
>(
    // cs builder
    builder: CsBuilder<TImpl, F, GC, TB>,
    // placement strategy (general purpose column、specialized column)
    placement_strategy: GatePlacementStrategy,
) -> CsBuilder<
    TImpl,
    F,
    (GateTypeEntry<F, Self, NextGateCounterWithoutParams>, GC),
    (
        Tool<ConstantToVariableMappingToolMarker, ConstantToVariableMappingTool<F>>,
        TB,
    ),
> {
    // we want to have a CS-global toolbox under some marker
    builder
        // 注册 gate
        .allow_gate(placement_strategy, (), None)
        // 注册辅助 tool，这里加入的是从 constant 映射到 variable 的 hashmap
        .add_tool::<ConstantToVariableMappingToolMarker, _>(
            ConstantToVariableMappingTool::<F>::with_capacity(16),
        )
}
``` 

每个 Gate 都实现了一个独特的功能函数。在 ConstantsAllocatorGate 中这个函数叫做 allocate_constant，功能是在电路中赋值 constant
观察 allocate_constant

```rust
pub fn allocate_constant<CS: ConstraintSystem<F>>(cs: &mut CS, constant_to_add: F) -> Variable {
    // 检查 cs builder 中是否注册了 ConstantsAllocatorGate
    debug_assert!(cs.gate_is_allowed::<Self>());
    // 检查 cs builder 中是否注册了 ConstantsAllocatorGate 的 tool
    let tooling: &mut ConstantToVariableMappingTool<F> = cs
        .get_static_toolbox_mut()
        .get_tool_mut::<ConstantToVariableMappingToolMarker, ConstantToVariableMappingTool<F>>()
        .expect("tool must be added");
    // 如果已经赋值过相同的 constant，那么可以在 tool 中找到相应的 variable，直接返回这个 variable
    if let Some(variable) = tooling.get(&constant_to_add).copied() {
        return variable;
    }
    drop(tooling);
    // 定义一个空的结果
    let output_variable = cs.alloc_variable_without_value();
    let tooling: &mut ConstantToVariableMappingTool<F> = cs
        .get_static_toolbox_mut()
        .get_tool_mut::<ConstantToVariableMappingToolMarker, ConstantToVariableMappingTool<F>>()
        .expect("tool must be added");
    // 在 tool 中记录 constant 对应的 variable
    let existing = tooling.insert(constant_to_add, output_variable);
    assert!(existing.is_none());
    drop(tooling);
    if <CS::Config as CSConfig>::WitnessConfig::EVALUATE_WITNESS {
        let value_fn = move |_inputs: [F; 0]| [constant_to_add];
        let dependencies = [];
        // 这个不是特别懂……
        cs.set_values_with_dependencies(
            &dependencies,
            &Place::from_variables([output_variable]),
            value_fn,
        );
    }
    // setup 阶段
    if <CS::Config as CSConfig>::SetupConfig::KEEP_SETUP == true {
        let gate = Self::new_to_enforce(output_variable, constant_to_add);
        // 将 gate 排布到电路中
        gate.add_to_cs(cs);
    }
    output_variable
}
```

下面我们可以看看 常量门的 `add_to_cs()` 函数的实现源码：

```rust
pub fn add_to_cs<F: SmallField, CS: ConstraintSystem<F>>(self, cs: &mut CS) {
    // 门必须被设置为允许
    debug_assert!(cs.gate_is_allowed::<Self>());

    // 只运行在 setup 阶段
    if <CS::Config as CSConfig>::SetupConfig::KEEP_SETUP == false {
        return;
    }

    // 对于 general purpose column 和 specialized column 有不同的 place 方案
    match cs.get_gate_placement_strategy::<Self>() {
        GatePlacementStrategy::UseGeneralPurposeColumns => {
            // 获取下一个可以放置新gate的行
            let offered_row_idx = cs.next_available_row();
            // 获取该行的最大长度
            let capacity_per_row = self.capacity_per_row(&*cs);
            // tooling是一个二元组，存放的是行列坐标
            let tooling: &mut NextGateCounterWithoutParams = cs
                .get_gates_config_mut()
                .get_aux_data_mut::<Self, _>()
                .expect("gate must be allowed");
            let (row, num_instances_already_placed) =
                find_next_gate_without_params(tooling, capacity_per_row, offered_row_idx);
                // 已经存在这样的门且门所在的行未满，则row != offered_row_idx，否则row == offered_row_idx
            drop(tooling);

            // now we can use methods of CS to inform it of low level operations
            let offset = num_instances_already_placed * PRINCIPAL_WIDTH;
            if offered_row_idx == row {
                // 当前是新行，则在当前行放置gate
                cs.place_gate(&self, row);
            }
            // 无论如何，都在当前行放入需要置换证明的variable
            cs.place_variable(self.variable_to_set, row, offset);
        }
        GatePlacementStrategy::UseSpecializedColumns {
            num_repetitions,
            share_constants: _,
        } => {
            // gate knows how to place itself
            let capacity_per_row = num_repetitions;
            let tooling: &mut NextGateCounterWithoutParams = cs
                .get_gates_config_mut()
                .get_aux_data_mut::<Self, _>()
                .expect("gate must be allowed");
            let (row, num_instances_already_placed) =
                find_next_specialized_gate_without_params(tooling, capacity_per_row);
            cs.place_gate_specialized(&self, num_instances_already_placed, row);
            cs.place_variable_specialized::<Self>(
                self.var_to_enforce,
                num_instances_already_placed,
                row,
                0,
            );
        }
    }
}
```

## 5. 常见电路门分析

### 5.1 FMA Gate

`fma_gate_without_constant.rs`


FMA Gate 可能是最常用的门之一，约束 `coeff_for_quadtaric_part * a * b + linear_term_coeff * c = out`，
并且有两种使用方式，第一种是自动计算输出：

```rust
/*
    pub fn compute_fma<CS: ConstraintSystem<F>>(
        cs: &mut CS,                    // 约束系统
        coeff_for_quadtaric_part: F,    // 二次项系数
        ab: (Variable, Variable),       // 二次项的两个乘数 a 和 b
        linear_term_coeff: F,           // 一次项系数
        c: Variable,                    // 一次项
    ) -> Variable
*/

let out = FmaGateInBaseFieldWithoutConstant::compute_fma(
    cs,
    coeff_for_quadtaric_part,
    (a, b),
    linear_term_coeff,
    c,
);
```

这种方式不用提前分配并计算 `out`， `compute_fma()` 得到的返回值就是 `out` ，生成约束的过程则内置于 `compute_fma`中。

下面是第二种方式：

```rust
let gate = FmaGateInBaseFieldWithoutConstant {
    params: FmaGateInBaseWithoutConstantParams {
        coeff_for_quadtaric_part,
        linear_term_coeff,
    },
    quadratic_part: (a, b),
    linear_part: c,
    rhs_part: out,      // 此处需要多给一个 rhs_part 值
};

gate.add_to_cs(cs);
```

这种方式需要显式给定`out`。

比如要验证两个变量`a`和`b`相等，那么我们就可以写如下代码：

```rust
// 设置一个常量 1
let one = verifier_cs.allocate_constant(GoldilocksField::ONE);

// 1 * a * 1 + 0 * 1 = b
let gate = FmaGateInBaseFieldWithoutConstant {
    params: FmaGateInBaseWithoutConstantParams {
        coeff_for_quadtaric_part: F::ONE,
        linear_term_coeff: F::ZERO,
    },
    quadratic_part: (a, one),
    linear_part: one,
    rhs_part: b,
};

gate.add_to_cs(&mut cs);
```

在 FMA Gate 的 `add_to_cs()` 代码实现中，我们可以看到 gate 在电路中排布的方式。

```rust
    pub fn add_to_cs<CS: ConstraintSystem<F>>(self, cs: &mut CS) {
        debug_assert!(cs.gate_is_allowed::<Self>());

        if <CS::Config as CSConfig>::SetupConfig::KEEP_SETUP == false {
            return;
        }

        let all_variables = [
            self.quadratic_part.0,
            self.quadratic_part.1,
            self.linear_part,
            self.rhs_part,
        ];

        match cs.get_gate_placement_strategy::<Self>() {
            GatePlacementStrategy::UseGeneralPurposeColumns => {
                let offered_row_idx = cs.next_available_row();
                let capacity_per_row = self.capacity_per_row(&*cs);
                let tooling: &mut HashMap<FmaGateInBaseWithoutConstantParams<F>, (usize, usize)> =
                    &mut cs
                        .get_gates_config_mut()
                        .get_aux_data_mut::<Self, FmaGateTooling<F>>()
                        .expect("gate must be allowed")
                        .1;
                let (row, num_instances_already_placed) =
                    find_next_gate(tooling, self.params, capacity_per_row, offered_row_idx);
                drop(tooling);

                // now we can use methods of CS to inform it of low level operations
                let offset = num_instances_already_placed * PRINCIPAL_WIDTH;
                if offered_row_idx == row {
                    cs.place_gate(&self, row);
                }
                cs.place_constants(
                    &[
                        self.params.coeff_for_quadtaric_part,
                        self.params.linear_term_coeff,
                    ],
                    row,
                    0,
                ); // this gate used same constants per row only
                assert_no_placeholder_variables(&all_variables);
                cs.place_multiple_variables_into_row(&all_variables, row, offset);
            }
            GatePlacementStrategy::UseSpecializedColumns {
                num_repetitions,
                share_constants: _,
            } => {
                // gate knows how to place itself
                let capacity_per_row = num_repetitions;
                let t: &mut FmaGateTooling<F> = cs
                    .get_gates_config_mut()
                    .get_aux_data_mut::<Self, FmaGateTooling<F>>()
                    .expect("gate must be allowed");

                let (next_available_row, tooling) = (&mut t.0, &mut t.1);
                let (row, num_instances_already_placed) = find_next_gate_specialized(
                    tooling,
                    next_available_row,
                    self.params,
                    capacity_per_row,
                );
                cs.place_gate_specialized(&self, num_instances_already_placed, row);
                cs.place_constants_specialized::<Self, 2>(
                    &[
                        self.params.coeff_for_quadtaric_part,
                        self.params.linear_term_coeff,
                    ],
                    num_instances_already_placed,
                    row,
                    0,
                ); // this gate used same constants per row only
                assert_no_placeholder_variables(&all_variables);
                cs.place_multiple_variables_into_row_specialized::<Self, 4>(
                    &all_variables,
                    num_instances_already_placed,
                    row,
                    0,
                );
            }
        }
    }
```

`find_next_gate_without_params`的作用就是判断当前`gate`的放置位置（如果需要放置的话）和`variable`放置的列。
如果当前`gate`还没有被放置到cs中，就新起一行放置当前`gate`和对应`variable`，否则就查看已经放置当前`gate`的行。
如果这一行还未填满`variable`，则填入`gate`对应的`variable`，否则也另起一行。
总结起来就是先按照从左到右排布 gate，直到行的末尾，再换到下一行开始排布。


此外，利用 FMA Gate 还可以创建一个计算求乘法逆的特殊算术门

```rust
    pub fn create_inversion_constraint<CS: ConstraintSystem<F>>(
        cs: &mut CS,
        variable_to_inverse: Variable,
        one_variable: Variable, // we need constant `1` (from any other consideration) as RHS
    ) -> Variable {
        ...
    }
```

### 5.2 ReductionGate

顾名思义，`ReductionGate`将输入的所有变量进行求和，返回求和结果。

```rust
/*
    pub fn reduce_terms<CS: ConstraintSystem<F>>(
        cs: &mut CS,                    // 约束系统
        reduction_constants: [F; N],    // 各项系数
        terms: [Variable; N],           // 各个项
    ) -> Variable
*/

// sum = 1 * a + 1 * b + 1 * c + 1 * d
let sum = ReductionGate::reduce_terms(
    cs,
    [F::ONE; 4],
    [
        a,
        b,
        c,
        d,
    ],
);
```

### 5.3 NopGate

空操作门，通常不会用到。

如果代码中出现`cs.pad_and_shrink();`，则需要添加`NopGate`。

### 5.4. PublicInputGate

公开输入门，将一个`variable`标记为`public input`。

```rust
// 首先定义一个variable
let thirty_five =
    cs.alloc_single_variable_from_witness(GoldilocksField::from_u64_unchecked(35));
// 定义PublicInputGate，并传入之前定义的variable
let gate = PublicInputGate::new(thirty_five);
// 将gate加入cs
gate.add_to_cs(&mut cs);
```

## 6. 电路代码示例：斐波那契数列

我们下面看一个采用多个门来证明斐波那契数列计算的例子。

```rust
fn simple_fibonacci() {
    type P = GoldilocksField;

    // 设置电路参数
    let geometry = CSGeometry {
        num_columns_under_copy_permutation: 8,
        num_witness_columns: 0,
        num_constant_columns: 2,
        max_allowed_constraint_degree: 8,
    };

    let max_variables = 512; // variable数量上限
    let max_trace_len = 128; // 电路表格的行数上限

    // 配置cs的函数
    fn configure<
        F: SmallField,
        T: CsBuilderImpl<F, T>,
        GC: GateConfigurationHolder<F>,
        TB: StaticToolboxHolder,
    >(
        builder: CsBuilder<T, F, GC, TB>,
    ) -> CsBuilder<T, F, impl GateConfigurationHolder<F>, impl StaticToolboxHolder> {
        
        // 在cs中加入fma门
        let builder = FmaGateInBaseFieldWithoutConstant::configure_builder(
            builder,
            GatePlacementStrategy::UseGeneralPurposeColumns,
        );
        
        // 在cs中加入public input门
        let builder = PublicInputGate::configure_builder(
            builder,
            GatePlacementStrategy::UseGeneralPurposeColumns,
        );

        // 在cs中加入空操作门，用于pad_and_shrink
        let builder =
            NopGate::configure_builder(builder, GatePlacementStrategy::UseGeneralPurposeColumns);

        builder
    }

    // cs builder: 约束系统的工厂类
    let builder_impl = CsReferenceImplementationBuilder::<GoldilocksField, P, DevCSConfig>::new(
        geometry,
        max_trace_len,
    );
    let builder = new_builder::<_, GoldilocksField>(builder_impl);

    let builder = configure(builder);
    let mut cs = builder.build(CircuitResolverOpts::new(max_variables));

    let mut previous_b = None;
    let mut previous_c = None;
    // 设置一个witness变量，并将它转化为variable
    let one = ConstantsAllocatorGate::allocate_constant(&mut cs, GoldilocksField::ONE);

    // 证明第n个fibonacci数为out
    let n = 9;
    let out = 34;

    // 循环 n - 2 轮
    for _ in 0..n - 2 {
        let a = if let Some(previous) = previous_b {
            // a 为上一轮的 b
            previous
        } else {
            // 初始化 a 为 1
            one
        };
        let b = if let Some(previous) = previous_c {
            // b 为上一轮的 c
            previous
        } else {
            // 初始化 b 为 1
            one
        };

        // c = 1 * (a * 1) + 1 * b
        // compute_fma自动计算c并生成约束
        let c: Variable = FmaGateInBaseFieldWithoutConstant::compute_fma(
            &mut cs,
            GoldilocksField::ONE,
            (a, one),
            GoldilocksField::ONE,
            b,
        );
        previous_b = Some(b);
        previous_c = Some(c);
    }

    let out = cs.alloc_single_variable_from_witness(GoldilocksField::from_u64_unchecked(out));
    let gate = PublicInputGate::new(out);
    gate.add_to_cs(&mut cs);

    if let Some(c) = previous_c {
        // 新增一个fma门，形成约束c = out
        // 这里用的是 gate.add_to_cs(&mut cs)，和之前的compute_fma不同
        let gate = FmaGateInBaseFieldWithoutConstant {
            params: FmaGateInBaseWithoutConstantParams {
                coeff_for_quadtaric_part: GoldilocksField::ONE,
                linear_term_coeff: GoldilocksField::ZERO,
            },
            quadratic_part: (c, one),
            linear_part: one,
            rhs_part: out,
        };

        gate.add_to_cs(&mut cs);
    } else {
        panic!("n must be at least 2");
    }

    // 补齐
    cs.pad_and_shrink();

    // 
}

```

## 7. 电路代码示例：多项式计算


```rust
    // 设置电路参数
    let geometry = CSGeometry {
        num_columns_under_copy_permutation: 8,
        num_witness_columns: 0,
        num_constant_columns: 3,
        max_allowed_constraint_degree: 8,
    };

    let max_variables = 512; // variable数量上限
    let max_trace_len = 128; // 电路表格的行数上限

    // 配置cs的函数
    fn configure<
        F: SmallField,
        T: CsBuilderImpl<F, T>,
        GC: GateConfigurationHolder<F>,
        TB: StaticToolboxHolder,
    >(
        builder: CsBuilder<T, F, GC, TB>,
    ) -> CsBuilder<T, F, impl GateConfigurationHolder<F>, impl StaticToolboxHolder> {
        // 在cs中加入constant门
        let builder = ConstantsAllocatorGate::configure_builder(
            builder,
            GatePlacementStrategy::UseGeneralPurposeColumns,
        );
        // 在cs中加入public input门
        let builder = PublicInputGate::configure_builder(
            builder,
            GatePlacementStrategy::UseGeneralPurposeColumns,
        );
        // 在cs中加入fma门
        let builder = FmaGateInBaseFieldWithoutConstant::configure_builder(
            builder,
            GatePlacementStrategy::UseGeneralPurposeColumns,
        );
        // 在cs中加入reduction门
        let builder = ReductionGate::<F, 3>::configure_builder(
            builder,
            GatePlacementStrategy::UseGeneralPurposeColumns,
        );
        // 在cs中加入空操作门，用于pad_and_shrink
        let builder =
            NopGate::configure_builder(builder, GatePlacementStrategy::UseGeneralPurposeColumns);

        builder
    }

    // cs builder: 约束系统的工厂类
    let builder_impl = CsReferenceImplementationBuilder::<GoldilocksField, P, DevCSConfig>::new(
        geometry,
        max_trace_len,
    );
    let builder = new_builder::<_, GoldilocksField>(builder_impl);

    let builder = configure(builder);
    let mut cs = builder.build(CircuitResolverOpts::new(max_variables));

    // 设置一个witness变量，并将它转化为variable
    let one = cs.allocate_constant(GoldilocksField::ONE);
    let five = cs.allocate_constant(GoldilocksField::from_u64_unchecked(5));

    // 设置public input
    let thirty_five =
        cs.alloc_single_variable_from_witness(GoldilocksField::from_u64_unchecked(35));
    let gate = PublicInputGate::new(thirty_five);
    gate.add_to_cs(&mut cs);

    // 设置 x = 3
    let x = cs.alloc_single_variable_from_witness(GoldilocksField::from_u64_unchecked(3));

    // x^2 = 1 * x * x + 0 * 1
    let x_square = FmaGateInBaseFieldWithoutConstant::compute_fma(
        &mut cs,
        GoldilocksField::ONE,
        (x, x),
        GoldilocksField::ZERO,
        one,
    );

    // x^3 = 1 * x^2 * x + 0 * 1
    let x_cube = FmaGateInBaseFieldWithoutConstant::compute_fma(
        &mut cs,
        GoldilocksField::ONE,
        (x_square, x),
        GoldilocksField::ZERO,
        one,
    );

    // x^3 + x + 5 == 35
    let gate = ReductionGate {
        params: ReductionGateParams {
            reduction_constants: [GoldilocksField::ONE; 3],
        },
        terms: [x_cube, x, five],
        reduction_result: thirty_five,
    };

    gate.add_to_cs(&mut cs);

    // padding
    cs.pad_and_shrink();
 
    // ...
```