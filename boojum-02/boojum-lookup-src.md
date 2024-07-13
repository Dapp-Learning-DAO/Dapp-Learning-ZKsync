# Boojum Lookup 源码分析

## 相关源码文件清单

- `src/cs/implementations/cslookup_placement.rs`
- `src/cs/implementations/lookup_argument_in_ext.rs`
- `src/cs/implementations/lookup_table.rs`
- `src/cs/gates/lookup_marker.rs`
- `src/cs/implementation/lookup_placement.rs`

## Lookup Argument 相关源码分析

我们先看 Lookup Argument 在 Prover 的算法部分。

第一部分这部分代码请见

- `src/cs/implementations/prover.rs`  Line: 426~515

其作用是关于 lookup argument 这个模块，Prover 需要构造的多项式，还有期间 Verifier 发送的随机挑战数 $\beta$ 与 $\gamma$。

下面列出这段代码及其解释：

```rust
let (
    (lookup_witness_encoding_polys, lookup_multiplicities_encoding_polys),
    lookup_beta,
    lookup_gamma,
) = if self.lookup_parameters != LookupParameters::NoLookup {

    // Verifier 发送一个随机数, beta
    // lookup argument related parts
    let lookup_beta = transcript.get_multiple_challenges_fixed::<2>();
    let lookup_beta = ExtensionField::<F, 2, EXT>::from_coeff_in_base(lookup_beta);

    // Verifier 发送另一个随机数, gamma
    let lookup_gamma = transcript.get_multiple_challenges_fixed::<2>();
    let lookup_gamma = ExtensionField::<F, 2, EXT>::from_coeff_in_base(lookup_gamma);

    //
    let contributions = match self.lookup_parameters {
        LookupParameters::NoLookup => {
            unreachable!()
        }
        LookupParameters::TableIdAsConstant { .. }
        | LookupParameters::TableIdAsVariable { .. } => {
            // exists by our setup
            let lookup_evaluator_id = 0;
            let _selector_subpath = setup_base
                .selectors_placement
                .output_placement(lookup_evaluator_id)
                .expect("lookup gate must be placed");

            let _columns_per_subargument = self.lookup_parameters.columns_per_subargument();

            // 暂不支持 lookup table 使用 general purpose columns 电路排布方式
            todo!()
        }
        a @ LookupParameters::UseSpecializedColumnsWithTableIdAsVariable { .. }
        | a @ LookupParameters::UseSpecializedColumnsWithTableIdAsConstant { .. } => {
            // 检查 lookup gate 的类型
            // ensure proper setup
            assert_eq!(
                self.evaluation_data_over_specialized_columns
                    .gate_type_ids_for_specialized_columns[0],
                std::any::TypeId::of::<LookupFormalGate>(),
                "we expect first specialized gate to be the lookup gate"
            );

            // 
            let (initial_offset, offset_per_repetition, _) = self
                .evaluation_data_over_specialized_columns
                .offsets_for_specialized_evaluators[0];
            assert_eq!(initial_offset.constants_offset, 0);

            if let LookupParameters::UseSpecializedColumnsWithTableIdAsConstant {
                share_table_id,
                ..
            } = a
            {
                if share_table_id {
                    assert_eq!(offset_per_repetition.constants_offset, 0);
                }
            }
            // 计算 A(x) 和 B(x)，下面这个函数是关键代码
            // extend to extension field simultaneously
            super::lookup_argument_in_ext::compute_lookup_poly_pairs_specialized(
                variables_columns.clone(),
                mutliplicities_columns.clone(),
                setup_base.constant_columns.clone(),
                setup_base.lookup_tables_columns.clone(),
                setup_base.table_ids_column_idxes.clone(),
                lookup_beta,
                lookup_gamma,
                self.parameters.num_columns_under_copy_permutation,
                a,
                worker,
                ctx,
            )
        }
    };

    (contributions, lookup_beta, lookup_gamma)
} else {
    let zero = ExtensionField::<F, 2, EXT>::ZERO;

    ((vec![], vec![]), zero, zero)
};
```

接下来，我们需要进一步深入到 `compute_lookup_poly_pairs_specialized()` 这个函数内部，看看 Prover 是如何构造 $A(X)$ 与 $B(X)$ 的。

下面是计算 $Ak(X)$ 与 $B(X)$ 的核心代码：

```rust
pub(crate) fn compute_lookup_poly_pairs_specialized<
    F: SmallField,
    P: field::traits::field_like::PrimeFieldLikeVectorized<Base = F>,
    EXT: FieldExtension<2, BaseField = F>,
    A: GoodAllocator,
    B: GoodAllocator,
>(
    // 所有的 variables 列（包含了 lookup arg variables 列）
    variables_columns: Vec<std::sync::Arc<GenericPolynomial<F, LagrangeForm, P, A>>, B>,
    // 与 lookup argument 相关的 multiplicity 列
    multiplicities_columns: Vec<std::sync::Arc<GenericPolynomial<F, LagrangeForm, P, A>>, B>,
    // 与 lookup argument 相关的 constant 列
    constant_polys: Vec<std::sync::Arc<GenericPolynomial<F, LagrangeForm, P, A>>, B>,
    // lookup_tables_columns 最后一列是 table_id
    lookup_tables_columns: Vec<std::sync::Arc<GenericPolynomial<F, LagrangeForm, P, A>>, B>,
    // 宽度为 1，唯一的元素是 table id 所在列下标
    table_id_column_idxes: Vec<usize>,
    // beta 和 gamma 是 lookup argument 中 Verifier 发送的随机数
    beta: ExtensionField<F, 2, EXT>,  // for denominator
    gamma: ExtensionField<F, 2, EXT>, // to aggregate columns
    // variables 结束列位置，也是 lookup table 起始位置
    variables_offset: usize,
    // lookup table 参数
    lookup_parameters: LookupParameters,
    // 用来并发计算的 worker 
    worker: &Worker,
    ctx: &mut P::Context,
) -> (
    // 返回两组多项式：
    //   1. lookup_witness_encoding_polys
    //   2. lookup_multiplicities_encoding_polys
    Vec<[GenericPolynomial<F, LagrangeForm, P, A>; 2], B>, // involves witness
    Vec<[GenericPolynomial<F, LagrangeForm, P, A>; 2], B>, // involve multiplicities
) {
    assert!(variables_columns.len() > 0);

    // 计算 trace 的长度
    let domain_size = variables_columns[0].domain_size();

    // 第一步: 计算 lookup argument 各种相关参数
    let (
        use_constant_for_table_id, // table id 作为 constant，还是 variable
        _share_table_id,           // table id 是否共享
        _width,            
        num_variable_columns_per_argument, // =table width (UseSpecializedColumnsWithTableIdAsConstant)
        total_num_columns_per_subargument, // =table width + 1, 每一个 lookup subargument 所占的表宽
        num_subarguments,                  // =num_repetitions, 总共有几个 sub-argument
    ) = match lookup_parameters {
        LookupParameters::UseSpecializedColumnsWithTableIdAsVariable {
            width,
            num_repetitions,
            share_table_id: _,
        } => (
            false,
            false,
            width as usize,
            // 比 UseSpecializedColumnsWithTableIdAsConstant 多一列，因为 table id 作为 variable
            width as usize + 1,
            width as usize + 1,
            num_repetitions,
        ),
        LookupParameters::UseSpecializedColumnsWithTableIdAsConstant {
            width,
            num_repetitions,
            share_table_id,
        } => {
            assert!(share_table_id);  // GY: table id 必须为共享模式

            (
                true,           // table id 作为 constant
                true,           // table id 因此可以共享
                width as usize,  // 表宽
                width as usize,  // 每一个 lookup sub-argument 带有的 variables 列数，因为 table id 共享，所以与表宽相等
                width as usize + 1,  // 每一个 lookup sub-argument 所占的表宽（列数），包含 table id 列
                num_repetitions,
            )
        }
        _ => unreachable!(),
    };

    // 第二步: 抽取 lookup argument 相关的 variables 列

    // we repeat the argument over betas and gammas sets
    // variables_offset == num_columns_under_copy_permutation
    // 抽取出所有的和 lookup arg 相关的 variables 列
    // 注意, lookup argument 使用的 variables columns 正好在普通 variables 列的（紧挨着）右侧
    let variables_columns_for_lookup = variables_columns[variables_offset
        ..(variables_offset + num_variable_columns_per_argument * num_subarguments)]
        .to_vec_in(B::default());

    // 把 beta, gamma 拆解为两个 Fp
    let beta_c0 = P::constant(beta.coeffs[0], ctx);
    let beta_c1 = P::constant(beta.coeffs[1], ctx);
    let _gamma_c0 = P::constant(gamma.coeffs[0], ctx);
    let _gamma_c1 = P::constant(gamma.coeffs[1], ctx);

    // 第三步：计算 T[]
    //    合并 table 所有的 columns，利用 gamma 的线性组合
    //    T[i] = aggregated_lookup_columns_inversed_c{0,1}

    let mut subarguments_witness_encoding_polys =
        Vec::with_capacity_in(num_subarguments, B::default());
    let mut subarguments_multiplicities_encoding_polys = Vec::with_capacity_in(1, B::default());

    //

    // 假如  t_i = (t_{i,0}, t_{i,1}, ..., t_{i,width}) 是 table 某一行的 keys-values-id
    //    
    // 计算 T[i] = 1 / (t_{i,0} + gamma * t_{i,1} + ... + gamma^width * t_{i,width})
    //
    // NOTE: table_id 放到了 variables_columns_for_lookup 的最后一列（最右边）

    // 计算 (1, gamma, gamma^2, ... , gamma^width)，注意包含了 table_id 列
    // NOTE: 计算结果为两个 Fp 向量，它们合在一起共同构成了 F_{p^2} 的向量
    let mut powers_of_gamma_c0 =
        Vec::with_capacity_in(total_num_columns_per_subargument, B::default());
    let mut powers_of_gamma_c1 =
        Vec::with_capacity_in(total_num_columns_per_subargument, B::default());
    // 分配一个初值为 1 的 F_{p^2} 对象
    let mut tmp = {
        use crate::field::traits::field::Field;

        ExtensionField::<F, 2, EXT>::ONE
    };
    powers_of_gamma_c0.push(P::constant(tmp.coeffs[0], ctx));
    powers_of_gamma_c1.push(P::constant(tmp.coeffs[1], ctx));

    // 在 F_{p^2} 上反复计算  tmp = tmp * gamma，并将 tmp 的两个系数放入之前分配的 Fp 向量
    for _ in 1..total_num_columns_per_subargument {
        crate::field::Field::mul_assign(&mut tmp, &gamma);

        powers_of_gamma_c0.push(P::constant(tmp.coeffs[0], ctx));
        powers_of_gamma_c1.push(P::constant(tmp.coeffs[1], ctx));
    }

    let mut aggregated_lookup_columns_c0 =
        Vec::with_capacity_in(domain_size / P::SIZE_FACTOR, A::default()); 
    let mut aggregated_lookup_columns_c1 =
        Vec::with_capacity_in(domain_size / P::SIZE_FACTOR, A::default());

    // 利用 worker 来并行计算  aggregated_lookup_columns_c0, aggregated_lookup_columns_c1
    //
    // | t0 | t1 | t2 | ... | tw | ==> | t0 + gamma * t1 + ... + gamma^w * tw |
    //
    worker.scope(domain_size / P::SIZE_FACTOR, |scope, chunk_size| {
        let mut subiterators = Vec::new_in(B::default());

        // idx: 0, 1, ... , domain_size / chunk_size
        for (idx, _) in aggregated_lookup_columns_c0.spare_capacity_mut()
            [..domain_size / P::SIZE_FACTOR]
            .chunks_mut(chunk_size)
            .enumerate()
        {
            let mut tmp = Vec::with_capacity_in(lookup_tables_columns.len(), B::default());
            // tmp 存的是 lookup_tables_columns 按照 chunk_size 行 chunk 之后，第 idx 个 chunk
            for src in lookup_tables_columns.iter() {
                // src 是单列 lookup table key-values
                // chunk 是第 idx 个 chunk
                let chunk = src
                    .storage
                    .chunks(chunk_size)
                    .nth(idx)
                    .expect("next chunk")
                    .iter();
                tmp.push(chunk);
            }
            // tmp 是一行的大小，因此 tmp.len() == total_num_columns_per_subargument == powers_of_gamma.len()
            assert_eq!(tmp.len(), powers_of_gamma_c0.len());
            assert_eq!(tmp.len(), powers_of_gamma_c1.len());
            subiterators.push(tmp);
        }

        // domain / chunk_size
        assert_eq!(
            aggregated_lookup_columns_c0.spare_capacity_mut()[..domain_size / P::SIZE_FACTOR]
                .chunks_mut(chunk_size)
                .len(),
            subiterators.len()
        );
        assert_eq!(
            aggregated_lookup_columns_c1.spare_capacity_mut()[..domain_size / P::SIZE_FACTOR]
                .chunks_mut(chunk_size)
                .len(),
            subiterators.len()
        );

        for ((dst_c0, dst_c1), src) in aggregated_lookup_columns_c0.spare_capacity_mut()
            [..domain_size / P::SIZE_FACTOR]
            .chunks_mut(chunk_size)
            .zip(
                aggregated_lookup_columns_c1.spare_capacity_mut()[..domain_size / P::SIZE_FACTOR]
                    .chunks_mut(chunk_size),
            )
            .zip(subiterators.into_iter())
        {
            let mut ctx = *ctx;
            let powers_of_gamma_c0 = &powers_of_gamma_c0;
            let powers_of_gamma_c1 = &powers_of_gamma_c1;

            // src.len() == total_num_columns_per_subargument == powers_of_gamma.len()
            assert_eq!(src.len(), powers_of_gamma_c0.len());
            assert_eq!(src.len(), powers_of_gamma_c1.len());

            scope.spawn(move |_| {
                let mut src = src;
                for (dst_c0, dst_c1) in dst_c0.iter_mut().zip(dst_c1.iter_mut()) {
                    let mut acc_c0 = beta_c0;
                    let mut acc_c1 = beta_c1;
                    for ((src, gamma_c0), gamma_c1) in src
                        .iter_mut()
                        .zip(powers_of_gamma_c0.iter())
                        .zip(powers_of_gamma_c1.iter())
                    {
                        let src = src.next().expect("table column element");
                        // acc += src * gamma^i
                        P::mul_and_accumulate_into(&mut acc_c0, src, gamma_c0, &mut ctx);
                        P::mul_and_accumulate_into(&mut acc_c1, src, gamma_c1, &mut ctx);
                    }

                    // aggregated_lookup_columns[i] = t_0 + gamma * t_1 + ... + beta
                    dst_c0.write(acc_c0);
                    dst_c1.write(acc_c1);
                }
            });
        }
    });

    // GY: aggregated_lookup_columns_c{0,1} 的长度应该等于 domain_size，排成了一纵列（从 F_{p^2} 视角）

    unsafe { aggregated_lookup_columns_c0.set_len(domain_size / P::SIZE_FACTOR) };
    unsafe { aggregated_lookup_columns_c1.set_len(domain_size / P::SIZE_FACTOR) };

    // 将 aggregated_lookup_columns 倒置为分母
    let mut aggregated_lookup_columns_inversed_c0 =
        P::vec_into_base_vec(aggregated_lookup_columns_c0);
    let mut aggregated_lookup_columns_inversed_c1 =
        P::vec_into_base_vec(aggregated_lookup_columns_c1);

    // 并行计算一组向量的 inverse
    batch_inverse_inplace_parallel_in_extension::<F, EXT, A>(
        &mut aggregated_lookup_columns_inversed_c0,
        &mut aggregated_lookup_columns_inversed_c1,
        worker,
    );
    let aggregated_lookup_columns_inversed_c0 =
        P::vec_from_base_vec(aggregated_lookup_columns_inversed_c0);
    let aggregated_lookup_columns_inversed_c1 =
        P::vec_from_base_vec(aggregated_lookup_columns_inversed_c1);

    // <接下面的代码片段>

```

witness_columns 按 chunk_size 行 chunk，得到了 subiterators

![subiterators](./imgs/subiterators.png)

witness_encoding_poly_c 也按 chunk_size 行 chunk，与 subiterators 进行 zip，两者的长度都是 (domain_size / P::SIZE_FACTOR) / chunk_size

![witness_encoding_poly_c](./imgs/witness_encoding_poly_c.png)

```rust
    // <接上面的代码片段>
    //
    // 第四步：计算 Ak[], k = 0, 1, ..., num_subarguments - 1
    //
    //    Ak[i] = 1 / (f_{i,0} + gamma * f_{i,1} + ... + gamma^width * f_{i,width})
    //
    // 假设总共有 `num_subarguments` 个 lookup sub-argument，那么会有 `num_subarguments` 个 lookup 向量，最后得到
    //   subarguments_witness_encoding_polys 这个多项式向量，其中每个元素为 witness_encoding_poly_c{0,1}，
    //   其向量元素类型为 F_{p^2}

    // we follow the same aproach as above - first prepare chunks, and then work over them
    // variables_columns_for_lookup: num_variable_columns_per_argument * num_subarguments
    for witness_columns in
        variables_columns_for_lookup.chunks_exact(num_variable_columns_per_argument)
    {
        let mut witness_encoding_poly_c0 =
            Vec::with_capacity_in(domain_size / P::SIZE_FACTOR, A::default());
        let mut witness_encoding_poly_c1 =
            Vec::with_capacity_in(domain_size / P::SIZE_FACTOR, A::default());

        worker.scope(domain_size / P::SIZE_FACTOR, |scope, chunk_size| {
            // prepare subiterators

            let mut subiterators = Vec::new_in(B::default());
            // idx: 0..domain_size / chunk_size
            for idx in 0..witness_columns[0].storage.chunks(chunk_size).len() {
                let mut tmp =
                    Vec::with_capacity_in(total_num_columns_per_subargument, B::default());
                // 与之前操作类似，将 witness_columns 按 chunk_size 行 chunk
                for src in witness_columns.iter() {
                    let chunk = src
                        .storage
                        .chunks(chunk_size)
                        .nth(idx)
                        .expect("next chunk")
                        .iter();
                    tmp.push(chunk);
                }
                // 如果 table_id_column_idxes 不为空
                if let Some(table_id_poly) = table_id_column_idxes.first().copied() {
                    // table id 必须为 constant
                    assert!(use_constant_for_table_id);
                    // table id 也被 chunk，并加入 tmp 后面
                    let chunk = constant_polys[table_id_poly]
                        .storage
                        .chunks(chunk_size)
                        .nth(idx)
                        .expect("next chunk")
                        .iter();
                    tmp.push(chunk);
                }
                assert_eq!(tmp.len(), powers_of_gamma_c0.len());
                assert_eq!(tmp.len(), powers_of_gamma_c1.len());
                subiterators.push(tmp);
            }

            // work with A poly only, compute denominator

            // domain_size / chunk_size
            assert_eq!(
                witness_encoding_poly_c0.spare_capacity_mut()[..domain_size / P::SIZE_FACTOR]
                    .chunks_mut(chunk_size)
                    .len(),
                subiterators.len()
            );
            assert_eq!(
                witness_encoding_poly_c1.spare_capacity_mut()[..domain_size / P::SIZE_FACTOR]
                    .chunks_mut(chunk_size)
                    .len(),
                subiterators.len()
            );

            for ((dst_c0, dst_c1), src) in witness_encoding_poly_c0.spare_capacity_mut()
                [..domain_size / P::SIZE_FACTOR]
                .chunks_mut(chunk_size)
                .zip(
                    witness_encoding_poly_c1.spare_capacity_mut()[..domain_size / P::SIZE_FACTOR]
                        .chunks_mut(chunk_size),
                )
                .zip(subiterators.into_iter())
            {
                let powers_of_gamma_c0 = &powers_of_gamma_c0[..];
                let powers_of_gamma_c1 = &powers_of_gamma_c1[..];
                let mut ctx = *ctx;
                // src.len() == total_num_columns_per_subargument == powers_of_gamma.len()
                assert_eq!(src.len(), powers_of_gamma_c0.len());
                assert_eq!(src.len(), powers_of_gamma_c1.len());

                scope.spawn(move |_| {
                    let mut src = src;
                    for (dst_c0, dst_c1) in dst_c0.iter_mut().zip(dst_c1.iter_mut()) {
                        let mut acc_c0 = beta_c0;
                        let mut acc_c1 = beta_c1;
                        for ((src, gamma_c0), gamma_c1) in src
                            .iter_mut()
                            .zip(powers_of_gamma_c0.iter())
                            .zip(powers_of_gamma_c1.iter())
                        {
                            // acc += src * gamma^i
                            let src = src.next().expect("witness column element");
                            P::mul_and_accumulate_into(&mut acc_c0, src, gamma_c0, &mut ctx);
                            P::mul_and_accumulate_into(&mut acc_c1, src, gamma_c1, &mut ctx);
                        }

                        dst_c0.write(acc_c0);
                        dst_c1.write(acc_c1);
                    }
                });
            }
        });

        unsafe { witness_encoding_poly_c0.set_len(domain_size / P::SIZE_FACTOR) };
        unsafe { witness_encoding_poly_c1.set_len(domain_size / P::SIZE_FACTOR) };

        let mut witness_encoding_poly_c0 = P::vec_into_base_vec(witness_encoding_poly_c0);
        let mut witness_encoding_poly_c1 = P::vec_into_base_vec(witness_encoding_poly_c1);

        // 倒转为分母，与上次倒转 aggregated_lookup_columns 不同，这次倒转的是 witness_encoding_poly
        batch_inverse_inplace_parallel_in_extension::<F, EXT, A>(
            &mut witness_encoding_poly_c0,
            &mut witness_encoding_poly_c1,
            worker,
        );
        let witness_encoding_poly_c0 = P::vec_from_base_vec(witness_encoding_poly_c0);
        let witness_encoding_poly_c1 = P::vec_from_base_vec(witness_encoding_poly_c1);

        // push the results
        let witness_encoding_poly_c0 = GenericPolynomial::from_storage(witness_encoding_poly_c0);
        let witness_encoding_poly_c1 = GenericPolynomial::from_storage(witness_encoding_poly_c1);

        subarguments_witness_encoding_polys
            .push([witness_encoding_poly_c0, witness_encoding_poly_c1]);
    }

    // GY: 第五步：计算 B[], multiplicities / aggregated_lookup_columns
    //      B[i] = M[i] * T[i], where M[i]: F_p, T[i]: F_{p^2}, and B[i]: F_{p^2}
    //
    //  Then,
    //      B[i] = {B[i]_c0, B[i]_c1} * M[i] = {B[i]_c0 * M[i], B[i]_c1 * M[i]}
    // 
    for multiplicity_column in multiplicities_columns.iter() {
        // A poly's denominator is ready, now we have simple elementwise pass

        let mut multiplicities_encoding_poly_c0 = aggregated_lookup_columns_inversed_c0.clone();
        let mut multiplicities_encoding_poly_c1 = aggregated_lookup_columns_inversed_c1.clone();

        worker.scope(
            multiplicities_encoding_poly_c0.len(),
            |scope, chunk_size| {
                for ((dst_c0, dst_c1), mults) in multiplicities_encoding_poly_c0
                    .chunks_mut(chunk_size)
                    .zip(multiplicities_encoding_poly_c1.chunks_mut(chunk_size))
                    .zip(multiplicity_column.storage.chunks(chunk_size))
                {
                    let mut ctx = *ctx;
                    scope.spawn(move |_| {
                        for ((dst_c0, dst_c1), mults) in
                            dst_c0.iter_mut().zip(dst_c1.iter_mut()).zip(mults.iter())
                        {
                            dst_c0.mul_assign(mults, &mut ctx);
                            dst_c1.mul_assign(mults, &mut ctx);
                        }
                    });
                }
            },
        );

        // push the results
        let multiplicities_encoding_poly_c0 =
            GenericPolynomial::from_storage(multiplicities_encoding_poly_c0);
        let multiplicities_encoding_poly_c1 =
            GenericPolynomial::from_storage(multiplicities_encoding_poly_c1);

        subarguments_multiplicities_encoding_polys.push([
            multiplicities_encoding_poly_c0,
            multiplicities_encoding_poly_c1,
        ]);
    }

    // 检查: subarguments_witness_encoding_polys 的长度与 num_subarguments 相等 
    //   subarguments_witness_encoding_polys 中每个元素长度应为 domain_size
    assert_eq!(subarguments_witness_encoding_polys.len(), num_subarguments);
    // 检查: multiplicities 多项式向量只有一个元素
    assert_eq!(subarguments_multiplicities_encoding_polys.len(), 1);

    // 检查:  Σ_{x in H} [A0(x) + A1(x) + ... + An(x)] ?= Σ_{x in H} B(x)
    if crate::config::DEBUG_SATISFIABLE == true {
        let mut a_sum_c0 = F::ZERO;
        let mut a_sum_c1 = F::ZERO;
        for a_poly in subarguments_witness_encoding_polys.iter() {
            for a in P::slice_into_base_slice(&a_poly[0].storage).iter() {
                a_sum_c0.add_assign(a);
            }
            for a in P::slice_into_base_slice(&a_poly[1].storage).iter() {
                a_sum_c1.add_assign(a);
            }
        }

        let mut b_sum_c0 = F::ZERO;
        let mut b_sum_c1 = F::ZERO;
        for b_poly in subarguments_multiplicities_encoding_polys.iter() {
            for b in P::slice_into_base_slice(&b_poly[0].storage).iter() {
                b_sum_c0.add_assign(b);
            }
            for b in P::slice_into_base_slice(&b_poly[1].storage).iter() {
                b_sum_c1.add_assign(b);
            }
        }

        if a_sum_c0 != b_sum_c0 || a_sum_c1 != b_sum_c1 {
            panic!(
                "Sumcheck fails with a = [{:?}, {:?}], b = [{:?}, {:?}]",
                a_sum_c0, a_sum_c1, b_sum_c0, b_sum_c1,
            );
        }
    }

    // 返回 {Ak(X)}_{k=0,..,n} 与 B(X)
    //    其中 Ak(X) 为 Ak[i] 的插值多项式，B(X) 为 B[i] 的插值多项式
    (
        subarguments_witness_encoding_polys,
        subarguments_multiplicities_encoding_polys,
    )
}
```

在 Prover 算法中，接下来的代码逻辑是对 $Ak(X)$ 与 $B{X}$ 进行 LDE 扩展

```rust
let second_stage_polys_storage = SecondStageProductsStorage::from_base_trace_ext(
    z_poly.clone(),
    intermediate_products,
    lookup_witness_encoding_polys,          // notice this
    lookup_multiplicities_encoding_polys,   // also this
    used_lde_degree,
    worker,
    ctx,
);
```

具体 LDE 的核心代码 `SecondStageProductsStorage::from_base_trace_ext()` 请参考 Boojum-FRI 的文档。 

下面的代码片段同样来自 Prover, 用于计算 $Ak(x)$ 与 $B(x)$ 在 $0$ 处的值。
根据 Univariate Sumcheck 的性质，任意多项式 $f(X)$ 在 domain H 上的求和等于 $f(0)$。
这个多项式计算过程通过 barycentric evaluation 算法分别求 Ak(0) 和 B(0)，
并保存入 all_polys_at_zero 多项式向量，交给后续的证明算法使用。

```rust
// evaluate from extension at zero
all_polys_at_zero.extend(
    second_stage_polys_storage
        // Σ^(n-1)_(i=0){1 / (beta + 1 * t[i][0] + gamma * t[i][1] + gamma^2 * t[i][2] + ... + gamma^k * t[i][k])}
        .lookup_witness_encoding_polys
        .iter()
        .map(|[a, b]| {
            [
                &a.storage[0].as_ref().storage,
                &b.storage[0].as_ref().storage,
            ]
        })
        .map(|[a, b]| evaluate_from_extension(a, b))
        .map(|el| ExtensionField::<F, 2, EXT>::from_coeff_in_base(el)),
);
all_polys_at_zero.extend(
    second_stage_polys_storage
        // Σ^(n-1)_(i=0){mults[i] / (beta + 1 * t[i][0] + gamma * t[i][1] + gamma^2 * t[i][2] + ... + gamma^k * t[i][k])}
        .lookup_multiplicities_encoding_polys
        .iter()
        .map(|[a, b]| {
            [
                &a.storage[0].as_ref().storage,
                &b.storage[0].as_ref().storage,
            ]
        })
        .map(|[a, b]| evaluate_from_extension(a, b))
        .map(|el| ExtensionField::<F, 2, EXT>::from_coeff_in_base(el)),
);
```

`evaluate_from_extension()` 的代码算法请参考 `src/cs/implementations/utils.rs` 中的
`barycentric_evaluate_extension_at_extension_for_bitreversed_parallel<F, EXT>()` 函数。

下面是 `src/cs/implementations/lookup_argument_in_ext.rs` 中的 `compute_quotient_terms_for_lookup_specialized` 的函数实现

```rust
pub(crate) fn compute_quotient_terms_for_lookup_specialized<
    F: PrimeField,
    EXT: FieldExtension<2, BaseField = F>,
    P: field::traits::field_like::PrimeFieldLikeVectorized<Base = F>,
    A: GoodAllocator,
    B: GoodAllocator,
>(
    witness: &WitnessStorage<F, P, A, B>,
    second_stage: &SecondStageProductsStorage<F, P, A, B>,
    setup: &SetupStorage<F, P, A, B>,
    beta: ExtensionField<F, 2, EXT>,
    gamma: ExtensionField<F, 2, EXT>,
    alphas: Vec<ExtensionField<F, 2, EXT>>,  // 聚合随机数
    table_id_column_idxes: Vec<usize>,
    column_elements_per_subargument: usize,
    num_subarguments: usize,                 // subargument 数量
    num_multiplicities_polys: usize,         // multiplicity 数量
    variables_offset: usize,        
    quotient_degree: usize,
    dst_c0: &mut ArcGenericLdeStorage<F, P, A, B>,
    dst_c1: &mut ArcGenericLdeStorage<F, P, A, B>,
    worker: &Worker,
    ctx: &mut P::Context,
) {

    // 需要聚合的多项式只有两类：(1) witness encoding poly (2) multiplicity poly
    assert_eq!(alphas.len(), num_subarguments + num_multiplicities_polys);
    // 强制要求 multiplicity poly 的数量为 `1`
    assert_eq!(num_multiplicities_polys, 1);
    // A(x) * (gamma^0 * column_0 + ... + gamma^n * column_n + beta) == lookup_selector

    // B(x) * (gamma^0 * column_0 + ... + gamma^n * column_n + beta) == multiplicity column

    let _gamma_c0 = P::constant(gamma.coeffs[0], ctx);
    let _gamma_c1 = P::constant(gamma.coeffs[1], ctx);

    let beta_c0 = P::constant(beta.coeffs[0], ctx);
    let beta_c1 = P::constant(beta.coeffs[1], ctx);

    // 取出 domain_size，即 trace 表格高度
    let domain_size = dst_c0.storage[0].domain_size();

    // 强制要求 table id 的列数为 `0` 或 `1`
    assert!(
        table_id_column_idxes.len() == 0 || table_id_column_idxes.len() == 1,
        "only specialized lookup with shared table ID in constants in supported, or no sharing, but got {:?} as table idxes for IDs",
        &table_id_column_idxes,
    );

    // 取出 variables_columns 中的所有 lookup_witness 列
    let variables_columns_for_lookup = witness.variables_columns
        [variables_offset..(variables_offset + column_elements_per_subargument * num_subarguments)]
        .to_vec_in(B::default());
    
    // capacity = 一个 subargument 的宽度（算上 table id 列）
    // this is our lookup width, either counted by number of witness columns only, or if one includes setup
    let capacity = column_elements_per_subargument + ((table_id_column_idxes.len() == 1) as usize);

    // 计算 (1, gamma, gamma^2, ..., gamma^c) 的值
    let mut powers_of_gamma_c0 = Vec::with_capacity_in(capacity, B::default());
    let mut powers_of_gamma_c1 = Vec::with_capacity_in(capacity, B::default());
    let mut tmp = {
        use crate::field::traits::field::Field;

        ExtensionField::<F, 2, EXT>::ONE
    };
    powers_of_gamma_c0.push(P::constant(tmp.coeffs[0], ctx));
    powers_of_gamma_c1.push(P::constant(tmp.coeffs[1], ctx));
    for _ in 1..capacity {
        crate::field::Field::mul_assign(&mut tmp, &gamma);

        powers_of_gamma_c0.push(P::constant(tmp.coeffs[0], ctx));
        powers_of_gamma_c1.push(P::constant(tmp.coeffs[1], ctx));
    }

    // for each term we form inputs, and then parallelize over them
    // 输出的多项式的长度为 (domain_size * quotient_degree)
    assert_eq!(dst_c0.outer_len(), quotient_degree);
    assert_eq!(dst_c1.outer_len(), quotient_degree);

    let iterators = dst_c0.compute_chunks_for_num_workers(worker.num_cores);
    // first precompute table aggregations

    // 首先计算聚合的 table 列 （长度为 quotient_degree）
    let aggregated_lookup_columns_c0 =
        setup.lookup_tables_columns[0].owned_subset_for_degree(quotient_degree);
    let aggregated_lookup_columns_c1 = ArcGenericLdeStorage::<F, P, A, B>::zeroed(
        aggregated_lookup_columns_c0.inner_len(),
        aggregated_lookup_columns_c0.outer_len(),
        A::default(),
        B::default(),
    );

    // 把 table 多项式的长度截取到 quotient_degree
    let mut other_lookup_columns =
        Vec::with_capacity_in(setup.lookup_tables_columns.len() - 1, B::default());
    setup.lookup_tables_columns[1..]
        .iter()
        .map(|el| el.subset_for_degree(quotient_degree))
        .collect_into(&mut other_lookup_columns);
    
    // 纵向 chunk，用于并行计算
    // 计算  t[i] = beta + t0[i] + gamma * t1[i] + gamma^2 * t2[i] + ... + gamma^c * tc[i] 
    // 写入  aggregated_lookup_columns_c{0, 1}
    // 
    // we access the memory exactly once
    let dst_chunks = aggregated_lookup_columns_c0.compute_chunks_for_num_workers(worker.num_cores);
    worker.scope(0, |scope, _| {
        // transpose other chunks
        for lde_iter in dst_chunks.into_iter() {
            let mut lde_iter = lde_iter;
            let powers_of_gamma_c0 = &powers_of_gamma_c0[1..];
            let powers_of_gamma_c1 = &powers_of_gamma_c1[1..];
            assert_eq!(powers_of_gamma_c0.len(), other_lookup_columns.len());
            assert_eq!(powers_of_gamma_c1.len(), other_lookup_columns.len());

            let mut ctx = *ctx;
            let mut aggregated_lookup_columns_c0 = aggregated_lookup_columns_c0.clone();
            let mut aggregated_lookup_columns_c1 = aggregated_lookup_columns_c1.clone();

            let other_lookup_columns = &other_lookup_columns;
            scope.spawn(move |_| {
                for _ in 0..lde_iter.num_iterations() {
                    let (outer, inner) = lde_iter.current();
                    let mut tmp_c0 = beta_c0;
                    let mut tmp_c1 = beta_c1;

                    for ((gamma_c0, gamma_c1), other) in powers_of_gamma_c0
                        .iter()
                        .zip(powers_of_gamma_c1.iter())
                        .zip(other_lookup_columns.iter())
                    {
                        P::mul_and_accumulate_into(
                            &mut tmp_c0,
                            gamma_c0,
                            &other.storage[outer].storage[inner],
                            &mut ctx,
                        );

                        P::mul_and_accumulate_into(
                            &mut tmp_c1,
                            gamma_c1,
                            &other.storage[outer].storage[inner],
                            &mut ctx,
                        );
                    }

                    // our "base" value for `aggregated_lookup_columns` already contains a term 1 * column_0,
                    // so we just add

                    unsafe {
                        std::sync::Arc::get_mut_unchecked(
                            &mut aggregated_lookup_columns_c0.storage[outer],
                        )
                        .storage[inner]
                            .add_assign(&tmp_c0, &mut ctx);
                    };

                    unsafe {
                        std::sync::Arc::get_mut_unchecked(
                            &mut aggregated_lookup_columns_c1.storage[outer],
                        )
                        .storage[inner]
                            .add_assign(&tmp_c1, &mut ctx);
                    };

                    lde_iter.advance();
                }
            });
        }
    });

    // 第二步: 计算聚合的 lookup_witness
    //   alpha^k * (a[k][i] * (f_0[k][i] + gamma * f_1[k][i] + ... + gamma^c * f_c[k][i]) - 1)
    //   , where k is the index of the subargument
    // now we can compute each term the same way, but identifying contributions
    let witness_encoding_poly_powers_of_alpha = &alphas[..num_subarguments];
    assert_eq!(
        witness_encoding_poly_powers_of_alpha.len(),
        variables_columns_for_lookup
            .chunks_exact(column_elements_per_subargument)
            .len()
    );

    for (idx, (alpha, vars_chunk)) in witness_encoding_poly_powers_of_alpha
        .iter()
        .zip(variables_columns_for_lookup.chunks_exact(column_elements_per_subargument))
        .enumerate()
    {
        let alpha_c0 = P::constant(alpha.coeffs[0], ctx);
        let alpha_c1 = P::constant(alpha.coeffs[1], ctx);

        // A(x) * (gamma^0 * column_0 + ... + gamma^n * column_n + beta) == lookup_selector
        let witness_encoding_poly_c0 =
            &second_stage.lookup_witness_encoding_polys[idx][0].subset_for_degree(quotient_degree);
        let witness_encoding_poly_c1 =
            &second_stage.lookup_witness_encoding_polys[idx][1].subset_for_degree(quotient_degree);

        let mut columns = Vec::with_capacity_in(capacity, B::default());
        for wit_column in vars_chunk.iter() {
            let subset = wit_column.subset_for_degree(quotient_degree);
            columns.push(subset);
        }

        // 聚合 table id 列
        if let Some(table_id_poly) = table_id_column_idxes.first().copied() {
            let subset = setup.constant_columns[table_id_poly].subset_for_degree(quotient_degree);
            columns.push(subset);
        }

        worker.scope(0, |scope, _| {
            // transpose other chunks
            for lde_iter in iterators.iter().cloned() {
                let mut lde_iter = lde_iter;
                let powers_of_gamma_c0 = &powers_of_gamma_c0[..];
                let powers_of_gamma_c1 = &powers_of_gamma_c1[..];
                let mut ctx = *ctx;
                let columns = &columns;
                assert_eq!(powers_of_gamma_c0.len(), columns.len());
                assert_eq!(powers_of_gamma_c1.len(), columns.len());

                let mut dst_c0 = dst_c0.clone(); // we use Arc, so it's the same instance
                let mut dst_c1 = dst_c1.clone(); // we use Arc, so it's the same instance
                scope.spawn(move |_| {
                    let one = P::one(&mut ctx);
                    for _ in 0..lde_iter.num_iterations() {
                        let (outer, inner) = lde_iter.current();
                        let mut tmp_c0 = beta_c0;
                        let mut tmp_c1 = beta_c1;

                        for ((gamma_c0, gamma_c1), other) in powers_of_gamma_c0
                            .iter()
                            .zip(powers_of_gamma_c1.iter())
                            .zip(columns.iter())
                        {
                            P::mul_and_accumulate_into(
                                &mut tmp_c0,
                                gamma_c0,
                                &other.storage[outer].storage[inner],
                                &mut ctx,
                            );

                            P::mul_and_accumulate_into(
                                &mut tmp_c1,
                                gamma_c1,
                                &other.storage[outer].storage[inner],
                                &mut ctx,
                            );
                        }
                        // mul by A(X)
                        mul_assign_vectorized_in_extension::<F, P, EXT>(
                            &mut tmp_c0,
                            &mut tmp_c1,
                            &witness_encoding_poly_c0.storage[outer].storage[inner],
                            &witness_encoding_poly_c1.storage[outer].storage[inner],
                            &mut ctx,
                        );

                        // subtract 1
                        tmp_c0.sub_assign(&one, &mut ctx);

                        // mul by alpha
                        mul_assign_vectorized_in_extension::<F, P, EXT>(
                            &mut tmp_c0,
                            &mut tmp_c1,
                            &alpha_c0,
                            &alpha_c1,
                            &mut ctx,
                        );

                        // 如果 dst_c0[0][i] != 0, 并且 domain 不是 coset，那么 panic
                        if crate::config::DEBUG_SATISFIABLE == true
                            && outer == 0
                            && tmp_c0.is_zero() == false
                        {
                            let mut normal_enumeration = inner.reverse_bits();
                            normal_enumeration >>= usize::BITS - domain_size.trailing_zeros();
                            panic!(
                                "A(x) term is invalid for index {} for subargument {}",
                                normal_enumeration, idx
                            );
                        }

                        // add into accumulator
                        unsafe {
                            std::sync::Arc::get_mut_unchecked(&mut dst_c0.storage[outer]).storage
                                [inner]
                                .add_assign(&tmp_c0, &mut ctx);
                        };
                        unsafe {
                            std::sync::Arc::get_mut_unchecked(&mut dst_c1.storage[outer]).storage
                                [inner]
                                .add_assign(&tmp_c1, &mut ctx);
                        };

                        lde_iter.advance();
                    }
                });
            }
        });
    }

    // now B poly

    // 计算 alpha^{num_subarguments} * (b[i] * t[i] - m[i]) 
    let multiplicities_encoding_poly_powers_of_alpha = &alphas[num_subarguments..];
    assert_eq!(
        multiplicities_encoding_poly_powers_of_alpha.len(),
        num_multiplicities_polys
    );
    for (idx, alpha) in multiplicities_encoding_poly_powers_of_alpha
        .iter()
        .enumerate()
    {
        let alpha_c0 = P::constant(alpha.coeffs[0], ctx);
        let alpha_c1 = P::constant(alpha.coeffs[1], ctx);
        // B(x) * (gamma^0 * column_0 + ... + gamma^n * column_n + beta) == multiplicity column
        let multiplicities_encoding_poly_c0 = &second_stage.lookup_multiplicities_encoding_polys
            [idx][0]
            .subset_for_degree(quotient_degree);
        let multiplicities_encoding_poly_c1 = &second_stage.lookup_multiplicities_encoding_polys
            [idx][1]
            .subset_for_degree(quotient_degree);
        // columns are precomputed, so we need multiplicity
        let multiplicity =
            witness.lookup_multiplicities_polys[idx].subset_for_degree(quotient_degree);
        worker.scope(0, |scope, _| {
            // transpose other chunks
            for lde_iter in iterators.iter().cloned() {
                let mut lde_iter = lde_iter;
                let mut ctx = *ctx;
                let mut dst_c0 = dst_c0.clone(); // we use Arc, so it's the same instance
                let mut dst_c1 = dst_c1.clone(); // we use Arc, so it's the same instance
                let multiplicity = &multiplicity;
                let aggregated_lookup_columns_c0 = &aggregated_lookup_columns_c0;
                let aggregated_lookup_columns_c1 = &aggregated_lookup_columns_c1;
                scope.spawn(move |_| {
                    for _ in 0..lde_iter.num_iterations() {
                        let (outer, inner) = lde_iter.current();
                        let mut tmp_c0 = aggregated_lookup_columns_c0.storage[outer].storage[inner];
                        let mut tmp_c1 = aggregated_lookup_columns_c1.storage[outer].storage[inner];
                        // mul by B(X)
                        mul_assign_vectorized_in_extension::<F, P, EXT>(
                            &mut tmp_c0,
                            &mut tmp_c1,
                            &multiplicities_encoding_poly_c0.storage[outer].storage[inner],
                            &multiplicities_encoding_poly_c1.storage[outer].storage[inner],
                            &mut ctx,
                        );

                        // subtract multiplicity
                        tmp_c0.sub_assign(&multiplicity.storage[outer].storage[inner], &mut ctx);

                        // mul by alpha
                        mul_assign_vectorized_in_extension::<F, P, EXT>(
                            &mut tmp_c0,
                            &mut tmp_c1,
                            &alpha_c0,
                            &alpha_c1,
                            &mut ctx,
                        );

                        // 如果 dst_c0[0][i] != 0, 并且 domain 不是 coset，那么 panic
                        if crate::config::DEBUG_SATISFIABLE == true
                            && outer == 0
                            && tmp_c0.is_zero() == false
                        {
                            let mut normal_enumeration = inner.reverse_bits();
                            normal_enumeration >>= usize::BITS - domain_size.trailing_zeros();
                            panic!(
                                "B(x) term is invalid for index {} for subargument {}",
                                normal_enumeration, idx
                            );
                        }

                        // add into accumulator
                        unsafe {
                            std::sync::Arc::get_mut_unchecked(&mut dst_c0.storage[outer]).storage
                                [inner]
                                .add_assign(&tmp_c0, &mut ctx);
                        };
                        unsafe {
                            std::sync::Arc::get_mut_unchecked(&mut dst_c1.storage[outer]).storage
                                [inner]
                                .add_assign(&tmp_c1, &mut ctx);
                        };

                        lde_iter.advance();
                    }
                });
            }
        });
    }
}
```