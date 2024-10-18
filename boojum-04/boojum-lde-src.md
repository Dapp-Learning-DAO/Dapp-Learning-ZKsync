# Boojum LDE 源码分析

## 概述


### 源码文件清单

1. `src/cs/implementations/polynomial/mod.rs`
2. `src/cs/implementations/polynomial/lde.rs`
3. `src/cs/implementations/polynomial_storage.rs`
   1. `src/cs/implementations/witness_storage.rs`
   2. `src/cs/implementations/setup_storage.rs`
4. `cs/implementations/utils.rs`

## GenericPolynomial 存储结构

代码文件: `src/cs/implementations/polynomial/mod.rs`

最关键的结构为 GenericPolynomial, 另一个相似的类型是 `Polynomial<F, FORM, A>`。


下面是这个结构体的定义

```rust
pub struct GenericPolynomial<
    F: PrimeField,
    FORM: PolynomialForm,
    P: field::traits::field_like::PrimeFieldLikeVectorized<Base = F>,
    A: GoodAllocator = Global,
> {
    #[serde(serialize_with = "crate::utils::serialize_vec_with_allocator")]
    #[serde(deserialize_with = "crate::utils::deserialize_vec_with_allocator")]
    pub storage: Vec<P, A>,
    pub _marker: std::marker::PhantomData<(F, P, FORM)>,
}
```

所谓的 `storage` 是指元素为 `PrimeFieldLikeVectorized` 的向量 `Vec<P, A>`

方法 `poly.domain_size()` 返回一个**按 2^k 对齐**的多项式长度，还乘上了 `P::SIZE_FACTOR` 这个系统参数。

方法 `poly.chunks(chunk_size)` 返回 `Vec<GenericPolynomialChunk>`，其中每一个 Chunk 指向原始 storage，并带上一个 range。最后一个 Chunk 的尺寸可能不足 `chunk_size`。

方法 `poly.chunk_into_subpolys_of_degree(degree)` 返回一个 MonomialForm Polynomial 向量，按照 degree 从低到高排列。这说明 Monomial Form 的系数排列也是从常数项系数到最高次项系数这样的顺序。


代码文件: `src/cs/implementations/polynomial/lde.rs`

第一个结构体类型为 `LdeIterator`。这个结构体目前不知道用在哪，故跳过。

第二个结构体类型为 `GenericLdeStorage<F, P, A, B>`，包含一个 `storage` 向量，元素为 `GenericPolynomial<F, BitreversedLagrangeForm, P, A>`。注意多项式为 RBO 序。

```rust
pub struct GenericLdeStorage<
    F: PrimeField,
    P: field::traits::field_like::PrimeFieldLikeVectorized<Base = F> = F,
    A: GoodAllocator = Global,
    B: GoodAllocator = Global,
> {
    pub storage: Vec<GenericPolynomial<F, BitreversedLagrangeForm, P, A>, B>,
}
```

方法 `ldestorage.inner_len()` 返回 `GenericPolynomial` 的长度。
方法 `ldestorage.outer_len()` 返回 `storage` 的长度。
构造方法 `ldestorage.from_single(values)` 从 values 构造一个 `GenericLdeStorage`。

第三个结构体为 `LdeParameters`，定义如下：

```rust
pub struct LdeParameters<F: PrimeField> {
    pub domain_size: usize,
    pub ldes_generator: F,
    pub additional_coset: F,
}
```

第四个结构体 `ArcGenericLdeStorage<F,P,A,B>`，

```rust
pub struct ArcGenericLdeStorage<
    F: PrimeField,
    P: field::traits::field_like::PrimeFieldLikeVectorized<Base = F>,
    A: GoodAllocator = Global,
    B: GoodAllocator = Global,
> {
    pub storage: Vec<Arc<GenericPolynomial<F, BitreversedLagrangeForm, P, A>>, B>,
}
```

其中 `storage` 是一个关于 `Arc<GenericLdeStorage<F, P, A, B>>` 的向量。

方法 `arc_ldestorage.inner_len()` 返回 `GenericPolynomial` 的长度。
方法 `arc_ldestorage.outer_len()` 返回 `storage` 的长度。

构造方法 `arc_ldestorage.subset_for_degree(degree)` 以浅拷贝的方式返回一个新的 `arc_ldestorage_new` ，其中的 storage 是原来的 storage 的子集 `storage[0..degree]`。
另一个构造方法 `arc_ldestorage.owned_subset_for_degree(degree)` 以深拷贝的方式返回一个新的 `arc_ldestorage_new` .

还有两个结构体 `GenericPolynomialLde` 与 `GenericLdeRowView` 暂时不知道在哪里用。
 
### `polynomial_storage.rs`

 `src/cs/implementations/polynomial_storage.rs`

所谓 `storage` 的含义是对若干组多项式，分别进行 LDE 计算之后产生的数据结构。包括 `WitnessStorage`，`SecondStageProductsStorage`, `SetupBaseStorage`, `SetupStorage` 与 `LookupSetupStorage` 。

它们有一个核心的方法 `from_base_trace()` 或者 `from_base_trace_ext()`，用来从原始的多项式数据结构中构造出 LDE 的数据结构。方法的过程包括两个部分
    1. 计算 `inverse_twiddles` 与 `forward_twiddles`
    2. 调用 `transform_from_arcs_to_lde_ext()` 或 `transform_from_arcs_to_lde()` 进行 LDE 计算

下面给出这些 storage 的结构体定义。

witness storage 这个名词的含义是电路的输入产生的 witness 部分。包括三类数据：
1. 变量列，variables_columns
2. advice列，witness_columns  
3. lookup 数量列，lookup_multiplicities_polys

第一个结构体为 `WitnessStorage<F, P, A, B>`，定义如下：

```rust
pub struct WitnessStorage<
    F: PrimeField,
    P: field::traits::field_like::PrimeFieldLikeVectorized<Base = F> = F,
    A: GoodAllocator = Global,
    B: GoodAllocator = Global,
> {
    // We store full LDEs of the original polynomials.
    // For those we can produce adapters to properly iterate over
    // future leafs of the oracles
    pub variables_columns: Vec<ArcGenericLdeStorage<F, P, A, B>, B>,
    pub witness_columns: Vec<ArcGenericLdeStorage<F, P, A, B>, B>,
    pub lookup_multiplicities_polys: Vec<ArcGenericLdeStorage<F, P, A, B>, B>,
}
```

方法 `flattened_source()` 返回一个关于多项式的 iterator，其中包含了全部三个多项式集合。

第二个结构体为 `SecondStageProductsStorage<F, P, A, B>`，定义如下：

```rust
pub struct SecondStageProductsStorage<
    F: PrimeField,
    P: field::traits::field_like::PrimeFieldLikeVectorized<Base = F> = F,
    A: GoodAllocator = Global,
    B: GoodAllocator = Global,
> {
    // We store full LDEs of the original polynomials.
    // For those we can produce adapters to properly iterate over
    // future leafs of the oracles
    pub z_poly: [ArcGenericLdeStorage<F, P, A, B>; 2],
    pub intermediate_polys: Vec<[ArcGenericLdeStorage<F, P, A, B>; 2], B>,
    pub lookup_witness_encoding_polys: Vec<[ArcGenericLdeStorage<F, P, A, B>; 2], B>,
    pub lookup_multiplicities_encoding_polys: Vec<[ArcGenericLdeStorage<F, P, A, B>; 2], B>,
}
```


方法 `flattened_source()` 返回一个关于多项式的 iterator，其中包含了全部四个多项式集合。

这一部分的数据包括：

1. `z_poly` 累乘器多项式
2. `intermediate_polys` 中间的部分乘积的多项式
3. `lookup_witness_encoding_polys` lookup 查询记录多项式
4. `lookup_multiplicities_encoding_polys` lookup multiplicities 查询数量多项式

第三个结构体为 `SetupBaseStorage<F, P, A, B>`，定义如下：

```rust
pub struct SetupBaseStorage<
    F: PrimeField,
    P: field::traits::field_like::PrimeFieldLikeVectorized<Base = F> = F,
    A: GoodAllocator = Global,
    B: GoodAllocator = Global,
> {
    // We store full LDEs of the original polynomials.
    // For those we can produce adapters to properly iterate over
    // future leafs of the oracles
    pub copy_permutation_polys: Vec<Arc<GenericPolynomial<F, LagrangeForm, P, A>>, B>,
    pub constant_columns: Vec<Arc<GenericPolynomial<F, LagrangeForm, P, A>>, B>,
    pub lookup_tables_columns: Vec<Arc<GenericPolynomial<F, LagrangeForm, P, A>>, B>,
    pub table_ids_column_idxes: Vec<usize>,
    pub selectors_placement: TreeNode,
}
```

这一部分数据是在 preprocessing 阶段进行预处理的多项式或者数据列，包括

1. `copy_permutation_polys` 复制约束多项式
2. `constant_columns` 常数列
3. `lookup_tables_columns` lookup 表列
4. `table_ids_column_idxes` 表 id 列索引
5. `selectors_placement` 选择器列

方法 `setup_storage.read_from_buffer()`

第四个结构体为 `SetupStorage<F,P,A,B>` ，与上面的结构体类似，但是没有 `selectors_placement`，换成了 `used_lde_degree`

```rust
pub struct SetupStorage<
    F: PrimeField,
    P: field::traits::field_like::PrimeFieldLikeVectorized<Base = F> = F,
    A: GoodAllocator = Global,
    B: GoodAllocator = Global,
> {
    // We store full LDEs of the original polynomials.
    // For those we can produce adapters to properly iterate over
    // future leafs of the oracles
    pub copy_permutation_polys: Vec<ArcGenericLdeStorage<F, P, A, B>, B>,
    pub constant_columns: Vec<ArcGenericLdeStorage<F, P, A, B>, B>,
    pub lookup_tables_columns: Vec<ArcGenericLdeStorage<F, P, A, B>, B>, // include the ID of the TABLE itself
    pub table_ids_column_idxes: Vec<usize>,
    pub used_lde_degree: usize,
}
```

方法 `flattened_source()` 返回一个关于多项式的 iterator，其中包含了前三个多项式集合。


第五个结构体 `LookupSetupStorage<F, P, A, B>` 的定义如下：

```rust
pub struct LookupSetupStorage<
    F: PrimeField,
    P: field::traits::field_like::PrimeFieldLikeVectorized<Base = F> = F,
    A: GoodAllocator = Global,
    B: GoodAllocator = Global,
> {
    pub table_polynomials: Vec<ArcGenericLdeStorage<F, P, A, B>, B>,
}
```

剩下还未 review 的结构体：
1. `AuxTraceInformation<F, P>`
2. `TraceHolder<F, P, A, B>`
3. `ProverTraceView<F, P, A, B>`
4. `TraceSourceDerivable<F, P>`
5. `SatisfiabilityCheckRowView`


另一个相关的文件是 `src/cs/implementations/witness_storage.rs`，里面包含了 LDE 的代码。

构造器方法 `WitnessStorage.from_base_trace()` 与 `WitnessStorage.from_base_trace_ext()` 函数的接口完全相同，只是内部实现的 copy 方式不同。目前不知道这个区分是何用意？

构造器方法 `SecondStageProductsStorage.from_base_trace_ext()` 是对若干个多项式进行 LDE 计算，产生 storage。

还有一个相关的文件是 `src/cs/implementations/setup_storage.rs`，里面只有一个方法 `SetupStorage.from_base_trace()`。


### FFT-LDE in `utils.rs`

这一节我们看 `cs/implementations/utils.rs` 这个文件中的关于 FFT-LDE 的算法部分。

第一个函数为 `pub fn precompute_twiddles_for_fft<F, P, A, INVERSED:bool>()`，用来预计算 $\omega^k$ 这些值，又被称为 twiddles。

```rust
pub fn precompute_twiddles_for_fft<
    F: PrimeField,
    P: field::traits::field_like::PrimeFieldLikeVectorized<Base = F>,
    A: GoodAllocator,
    const INVERSED: bool,
>(
    fft_size: usize,
    worker: &Worker,
    _ctx: &mut P::Context,
) -> Vec<P, A> {
    debug_assert!(fft_size.is_power_of_two());

    // 计算 omega, omega^N = 1, where N = fft_size
    let mut omega = domain_generator_for_size::<F>(fft_size as u64);
    if INVERSED {
        omega = omega
            .inverse()
            .expect("must always exist for domain generator");
    }

    // 检验 omega 确实是 N-th root of unity
    assert_eq!(omega.pow_u64(fft_size as u64), F::ONE);
    for i in 1..fft_size {
        assert_ne!(omega.pow_u64(i as u64), F::ONE);
    }

    // 计算 omega^k, k = 0, 1, 2, ..., N/2-1
    let num_powers = fft_size / 2;
    // MixedGL can have up to 16 elements, depending on implementation, so we can't
    // have less than that.
    let mut powers = materialize_powers_parallel::<F, P, A>(
        omega,
        std::cmp::max(num_powers, P::SIZE_FACTOR),
        worker,
    );

    // 把 powers 按照 RBO 排序
    // Items beyond `num_powers` are dead weight.
    bitreverse_enumeration_inplace(&mut powers[0..num_powers]);

    // 把 Vec<F, A> 转换成 Vec<P, A>
    P::vec_from_base_vec(powers)
}
```

第二个函数为 `precompute_twiddles_for_fft_natural()`，与上面的函数大体一样，只是没有 `bitreverse_enumeration_inplace()` 这一步。

函数 `transform_from_trace_to_lde(vec, lde_degree, twd1, twd2, ...)` 是一个非常关键的函数，用来将原始的多项式数据结构转换成 LDE 的数据结构。函数内部又调用 `transform_raw_storages_to_lde()`，完成 LDE 的计算。

下面看看 `transform_raw_storages_to_lde<F, P, A, B>` 的定义：

```rust
pub(crate) fn transform_raw_storages_to_lde<
    F: SmallField,
    P: field::traits::field_like::PrimeFieldLikeVectorized<Base = F>,
    A: GoodAllocator,
    B: GoodAllocator,
>(
    trace_columns: Vec<Vec<P, A>, B>,
    domain_size: usize,
    lde_degree: usize,
    inverse_twiddles: &P::InverseTwiddles<A>,
    forward_twiddles: &P::Twiddles<A>,
    worker: &Worker,
    ctx: &mut P::Context,
) -> Vec<ArcGenericLdeStorage<F, P, A, B>, B> {
    assert!(lde_degree.is_power_of_two());
    assert!(lde_degree > 1);

    debug_assert_eq!(domain_size, trace_columns[0].len() * P::SIZE_FACTOR);

    let _num_polys = trace_columns.len();

    let _now = std::time::Instant::now();

    // 并行计算 IFFT，得到 "系数式"
    // IFFT to get monomial form
    let mut jobs: Vec<Vec<P, A>, B> = trace_columns;
    worker.scope(jobs.len(), |scope, chunk_size| {
        for chunk in jobs.chunks_mut(chunk_size) {
            let mut ctx = *ctx;
            scope.spawn(move |_| {
                for poly in chunk.iter_mut() {
                    P::ifft_natural_to_natural(poly, F::ONE, inverse_twiddles, &mut ctx);
                }
            });
        }
    });

    // log!("{} iFFTs taken {:?}", num_polys, now.elapsed());
    // 通过 "系数式" 来计算 LDE
    transform_monomials_to_lde(jobs, domain_size, lde_degree, forward_twiddles, worker, ctx)
}
```

```rust
pub(crate) fn transform_monomials_to_lde<
    F: SmallField,
    P: field::traits::field_like::PrimeFieldLikeVectorized<Base = F>,
    A: GoodAllocator,
    B: GoodAllocator,
>(
    trace_columns: Vec<Vec<P, A>, B>,
    domain_size: usize,
    lde_degree: usize,
    forward_twiddles: &P::Twiddles<A>,
    worker: &Worker,
    ctx: &mut P::Context,
) -> Vec<ArcGenericLdeStorage<F, P, A, B>, B> {
    assert!(lde_degree.is_power_of_two());
    assert!(lde_degree > 1);

    let num_polys = trace_columns.len();

    debug_assert_eq!(domain_size, trace_columns[0].len() * P::SIZE_FACTOR);

    // 计算 lde_size = LDE 矩阵的大小，矩阵 = N * D, 
    //    N = domain_size, D = lde_degree
    let lde_size = domain_size * lde_degree;
    // 计算 LDE 矩阵 domain 的生成元 omega 
    let coset = domain_generator_for_size::<F>(lde_size as u64);
    debug_assert!({
        let outer_omega = domain_generator_for_size::<F>(domain_size as u64);
        let tmp = coset.pow_u64(lde_degree as u64);

        outer_omega == tmp
    });

    // 计算 F 的生成元 g
    // 即可以在 g*w^i 上进行 LDE，也可以直接在 w^i 上进行 LDE
    let multiplicative_generator = F::multiplicative_generator();

    // 计算 coset = (1, omega, omega^2, omega^3, ..., omega^{D-1})
    //    然后对 coset 进行 RBO 排序
    // 现在每一个 column 正好是一个多项式的 Monomial Form
    // now all polys are in the Monomial form, so let's LDE them
    let mut powers_of_coset = materialize_powers_serial::<F, A>(coset, lde_degree);
    // powers_of_coset = {1, omega, omega^2, omega^3, ...}
    bitreverse_enumeration_inplace(&mut powers_of_coset);
    let powers_of_coset_ref = &powers_of_coset[..];

    // 每一个 lde-coset 对应一个 job
    let jobs_per_coset = trace_columns.len();
    // we will create a temporary placeholder of vectors for more even CPU load
    // all_ldes 是一个长度为 lde_size 的数组，用来放置所有 LDE 的矩阵
    let mut all_ldes = Vec::with_capacity_in(trace_columns.len() * lde_degree, B::default());
    // 在 all_ldes 中重复放置 所有的原始多项式
    //   假设 Trace = (W1, W2, W3, W4)
    //   LDE  = [(W1, W2, W3, W4), (W1, W2, W3, W4), ..., (W1, W2, W3, W4)] 
    //  omega = [ w^0            ,  w^1,           , ...,  w^{D-1}        ]
    for _ in 0..(lde_degree - 1) {
        all_ldes.extend_from_slice(&trace_columns);
    }
    all_ldes.extend(trace_columns);

    // 开始并行计算 LDE
    worker.scope(all_ldes.len(), |scope, chunk_size| {
        for (chunk_idx, chunk) in all_ldes.chunks_mut(chunk_size).enumerate() {
            let mut ctx = *ctx;
            scope.spawn(move |_| {
                for (idx, poly) in chunk.iter_mut().enumerate() {
                    let poly_idx = chunk_idx * chunk_size + idx;
                    let coset_idx = poly_idx / jobs_per_coset;
                    // 设置 coset 为 omega^j,  j 为列数
                    let mut coset = powers_of_coset_ref[coset_idx];
                    if crate::config::DEBUG_SATISFIABLE == false {
                        coset.mul_assign(&multiplicative_generator);
                    }
                    debug_assert!(poly.as_ptr().addr() % std::mem::align_of::<P>() == 0);
                    // 计算 LDE，注意！每一列按 RBO 排序，
                    P::fft_natural_to_bitreversed(poly, coset, forward_twiddles, &mut ctx);
                }
            });
        }
    });

    // transpose them back. In "all_ldes" we have first coset for every poly, then next one for every, etc
    // and we need to place them into individual cosets for into poly
    let mut columns = Vec::with_capacity_in(num_polys, B::default());
    columns.resize(
        num_polys,
        ArcGenericLdeStorage::empty_with_capacity_in(lde_degree, B::default()),
    );

    // 把 all_ldes 中的多项式重新整理到 columns 中
    // columns = [(W1, W1, W1, W1), (W2, W2, W2, W2), ..., (W4, W4, W4, W4)]
    for (idx, el) in all_ldes.into_iter().enumerate() {
        let dst_poly_idx = idx % num_polys;
        let as_polynomial = GenericPolynomial::from_storage(el);
        columns[dst_poly_idx]
            .storage
            .push(std::sync::Arc::new(as_polynomial));
    }

    // log!("{} LDEs of degree {} taken {:?}", num_polys, lde_degree, now.elapsed());

    columns
}
```