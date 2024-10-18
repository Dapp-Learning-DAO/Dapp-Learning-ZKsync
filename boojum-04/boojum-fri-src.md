# Boojum FRI 源码分析

## 导读

Boojum FRI 实现了基于 Goldilocks 有限域的 FRI 协议算法。FRI 协议是一种用于验证多项式的算法，它的核心思想是通过多次折叠多项式，从而减少多项式的长度，最终验证多项式的正确性。Boojum FRI 的实现包括了多项式的存储结构，LDE 计算，FRI 协议的计算等。

<!-- ### 测试用例 -->

### 源码文件清单

**FRI 代码**

- `src/cs/implementations/fri/mod.rs` 
- `src/cs/implementations/prover.rs`

<!-- ### FRI 协议流程 -->


### 阅读顺序

建议先看 Polynomial 模块，熟悉多项式在 Prover 内存中的存储结构，然后看 FRI 相关代码

## FRI 协议源码分析


Prover 的第五步是证明 openings 的正确性，这将使用 FRI 协议来实现一个 多项式承诺方案，即 FRI-PCS，
其核心思想是将 $f(X)$ 在 $X=z_{f,0},\ldots z_{f,k-1}$ 个点处的值。

我们先看看如何证明一个多项式 $f(X)$ 在 $X=z$ 处打开。先构造一个虚拟多项式 Oracle, $q(X)$

$$
q(X) = \frac{f(X) - f(z)}{(X- z)}
$$

假设 $\deg{(f)}<N$，那么我们可以利用 FRI 协议对 $q(X)$ 进行 low-degree test，证明 $\deg{(g)}<N$。



### 计算 Quotient Polynomial

Boojum prover 需要证明四类多项式 openings，
1. $f(X)$ 在 $X=\zeta$ 处的值
2. $f(X)$ 在 $X=\omega\cdot \zeta$ 处的值
3. $f(X)$ 在 $X=0$ 处的取值
4. $f(X)$ 在 public inputs 处的取值




### FRI Commit 算法

先看 `fn compute_fri_schedule()` 这个函数，计算 FRI 每一轮的折叠过程。
这个函数的主要功能是计算安全参数，然后计算每一轮的折叠维度。根据四个参数：

1. security_bits: u32,
2. cap_size: usize,
3. pow_bits: u32,
4. rate_log_two: u32,
5. initial_degree_log_two: u32,

计算下面的安全参数：
1. new_pow_bits,
2. num_queries,
3. interpolation_log2s_schedule,
4. final_expected_degree

接下来就是 `fn do_fri()`，它的主要参数如下：
1. `rs_code_word_c0` 与 `rs_code_word_c1`，待证明的 F^2 上的多项式，分为 c0, c1 两个分量
2. `interpolation_log2s_schedule` 每一层的折叠维度
3. `lde_degree` LDE 的 blowup factor
4. `cap_size` merkle tree 的 cap size

1. 计算
```rust
pub fn do_fri<
    F: SmallField,
    P: field::traits::field_like::PrimeFieldLikeVectorized<Base = F>,
    EXT: FieldExtension<2, BaseField = F>,
    ...
>(
    rs_code_word_c0: ArcGenericLdeStorage<F, P, A, B>,
    rs_code_word_c1: ArcGenericLdeStorage<F, P, A, B>,
    transcript: &mut T,
    interpolation_log2s_schedule: Vec<usize>,
    lde_degree: usize,
    cap_size: usize,
    worker: &Worker,
    ctx: &mut P::Context,
) -> FriOracles<F, H, A, B, 2> {

    // outer_len = lde_degree
    // inner_len = domain_size
    debug_assert_eq!(rs_code_word_c0.outer_len(), rs_code_word_c1.outer_len());
    debug_assert_eq!(rs_code_word_c0.inner_len(), rs_code_word_c1.inner_len());

    // 第一步：计算 final_degree 
    let full_size = rs_code_word_c0.outer_len() * rs_code_word_c0.inner_len() * P::SIZE_FACTOR;
    let degree = full_size / lde_degree;
    let mut final_degree = degree;
    for interpolation_log2 in interpolation_log2s_schedule.iter() {
        let factor = 1usize << interpolation_log2;
        final_degree /= factor;
    }

    assert!(final_degree > 0);

    // 第二步：计算第一层多项式 f0 的 commitment，被称为 base_oracle 
    let mut base_sources = Vec::with_capacity_in(2, B::default());
    base_sources.push(rs_code_word_c0.clone());
    base_sources.push(rs_code_word_c1.clone());

    let base_oracle_elements_per_leaf = interpolation_log2s_schedule[0];

    let fri_base_oracle =
        oracle::merkle_tree::MerkleTreeWithCap::<F, H, A, B>::construct_by_chunking(
            base_sources,
            1 << base_oracle_elements_per_leaf,
            cap_size,
            worker,
        );

    transcript.witness_merkle_tree_cap(&fri_base_oracle.get_cap());

    // 第三步：完成顶层多项式的折叠
    let mut intermediate_oracles = Vec::new_in(B::default());
    let mut intermediate_sources = Vec::new_in(B::default());

    // 3.1：计算 roots (twiddles)，用来计算 IFFT，长度为 full_size 的一半
    //  NOTE: roots 是按 RBO 顺序排列
    let roots = precompute_twiddles_for_fft::<F, P, A, true>(full_size, worker, ctx);
    let roots = P::vec_into_base_vec(roots);

    // 计算 coset = primitive element，与 (1/coset)
    let mut coset_inverse = F::multiplicative_generator().inverse().unwrap();

    // we have to manually unroll 1st loop due to type dependency
    let mut it = interpolation_log2s_schedule.into_iter();

    // 计算第一轮的折叠维度
    let reduction_degree_log_2 = it.next().unwrap();

    // 计算 c0_values, c1_values 为第一轮折叠后的结果
    // 第一轮折叠调用的是 `interpolate_independent_cosets()`
    // 第一轮折叠时，storage 的存放是按矩阵的方式，更容易并行执行
    let (c0_values, c1_values) = {
        log!("Fold degree by {}", 1 << reduction_degree_log_2);
        assert!(reduction_degree_log_2 > 0);
        assert!(reduction_degree_log_2 < 4);

        let c0 = transcript.get_challenge();
        let c1 = transcript.get_challenge();

        let mut challenge_powers = Vec::with_capacity(reduction_degree_log_2);
        challenge_powers.push((c0, c1));
        use crate::field::ExtensionField;
        let as_extension = ExtensionField::<F, 2, EXT> {
            coeffs: [c0, c1],
            _marker: std::marker::PhantomData,
        };

        // `current` 的作用是产生 (a, a^2, a^4 ...) 这样的序列 
        let mut current = as_extension;

        for _ in 1..reduction_degree_log_2 {
            current.square();
            let [c0, c1] = current.into_coeffs_in_base();
            challenge_powers.push((c0, c1));
        }

        // now interpolate as described above

        let (c0, c1) = interpolate_independent_cosets::<F, P, EXT, A, B>(
            rs_code_word_c0.clone(),
            rs_code_word_c1.clone(),
            reduction_degree_log_2,
            &roots,
            challenge_powers,
            lde_degree,
            &mut coset_inverse,
            worker,
            ctx,
        );

        (c0, c1)
    };

    // 保存第一轮的折叠结果 (c0_values, c1_values)
    intermediate_sources.push((c0_values, c1_values));

    // 第四步：继续折叠，直到最后一轮
    // 调用 `interpolate_flattened_cosets()`
    for reduction_degree_log_2 in it {
        log!("Fold degree by {}", 1 << reduction_degree_log_2);

        // 确保 折叠次数是，2, 4, 8 这三种可能性
        assert!(reduction_degree_log_2 > 0);
        assert!(reduction_degree_log_2 < 4);

        // 取出上一轮的折叠结果，并构造 ORACLE(即 merkle tree)
        // make intermediate oracle for the next folding
        let mut sources = Vec::with_capacity_in(2, B::default());
        let (c0_values, c1_values) = intermediate_sources
            .last()
            .expect("previous folding result exists");
        sources.push(c0_values);
        sources.push(c1_values);
        let intermediate_oracle =
            MerkleTreeWithCap::<F, H, A, B>::construct_by_chunking_from_flat_sources(
                &sources,
                1 << reduction_degree_log_2,
                cap_size,
                worker,
            );

        transcript.witness_merkle_tree_cap(&intermediate_oracle.get_cap());

        // 保存上一轮的折叠向量的 ORACLE (即 merkle tree) 
        intermediate_oracles.push(intermediate_oracle);
        // compute next folding

        // 获取下一轮的折叠因子 c=(c0, c1)，并计算 (c, c^2, c^4)
        let c0 = transcript.get_challenge();
        let c1 = transcript.get_challenge();

        let mut challenge_powers = Vec::with_capacity(reduction_degree_log_2);
        challenge_powers.push((c0, c1));
        use crate::field::ExtensionField;
        let as_extension = ExtensionField::<F, 2, EXT> {
            coeffs: [c0, c1],
            _marker: std::marker::PhantomData,
        };

        let mut current = as_extension;

        for _ in 1..reduction_degree_log_2 {
            current.square();
            let [c0, c1] = current.into_coeffs_in_base();
            challenge_powers.push((c0, c1));
        }

        // now interpolate as described above

        let (c0_source, c1_source) = intermediate_sources.last().cloned().unwrap();

        // 计算新的折叠
        let (new_c0, new_c1) = interpolate_flattened_cosets::<F, EXT, A>(
            c0_source,
            c1_source,
            reduction_degree_log_2,
            &roots,
            challenge_powers,
            lde_degree,
            &mut coset_inverse,
            worker,
        );

        intermediate_sources.push((new_c0, new_c1));
    }

    // we can now interpolate the last sets to get monomial forms

    log!("Interpolating low degree poly");

    // 第五步：根据最后一轮的计算结果，计算最终多项式
    let (mut c0_source, mut c1_source) = intermediate_sources.last().cloned().unwrap();

    //  5.1 反向 RBO 排序
    bitreverse_enumeration_inplace(&mut c0_source);
    bitreverse_enumeration_inplace(&mut c1_source);

    //  5.2 计算 IFFT，得到最终多项式的 "系数形式"
    let coset = coset_inverse.inverse().unwrap();
    // IFFT our presumable LDE of some low degree poly
    let fft_size = c0_source.len();
    crate::fft::ifft_natural_to_natural(&mut c0_source, coset, &roots[..fft_size / 2]);
    crate::fft::ifft_natural_to_natural(&mut c1_source, coset, &roots[..fft_size / 2]);

    assert_eq!(final_degree, fft_size / lde_degree);

    // 5.3 检查多项式的最高次数确实等于 `final_degree`
    if crate::config::DEBUG_SATISFIABLE == false {
        for el in c0_source[final_degree..].iter() {
            assert_eq!(*el, F::ZERO);
        }

        for el in c1_source[final_degree..].iter() {
            assert_eq!(*el, F::ZERO);
        }
    }

    // 5.4 把最终多项式的系数形式添加到 transcript 中
    // add to the transcript
    transcript.witness_field_elements(&c0_source[..final_degree]);
    transcript.witness_field_elements(&c1_source[..final_degree]);

    // now we should do some PoW and we are good to go

    let monomial_form_0 = c0_source[..(fft_size / lde_degree)].to_vec_in(A::default());
    let monomial_form_1 = c1_source[..(fft_size / lde_degree)].to_vec_in(A::default());

    log!(
        "FRI for base size 2^{} is done over {:?}",
        full_size.trailing_zeros(),
        now.elapsed()
    );

    FriOracles {
        base_oracle: fri_base_oracle,
        leaf_sources_for_intermediate_oracles: intermediate_sources,
        intermediate_oracles,
        monomial_forms: [monomial_form_0, monomial_form_1],
    }
}
```

下面我们分析下 `interpolate_independent_cosets()` 函数与 `interpolate_flattened_cosets()` 函数的实现。前者仅用来对 codeword 的第一次折叠，后者用来对 codeword 的后面的多次折叠。另外请注意，对 `interpolate_independent_cosets` 调用一次，比如折叠次数为 8（总共 3 次 2-to-1 折叠），那么第一次折叠后，该函数还会继续调用 `interpolate_flattened_cosets` 来完成剩下的 2 次折叠。

> TODO: 解释第一次折叠为何特殊

```rust
fn interpolate_independent_cosets<
    F: SmallField,
    P: field::traits::field_like::PrimeFieldLikeVectorized<Base = F>,
    EXT: FieldExtension<2, BaseField = F>,
    ...
>(
    c0_source: ArcGenericLdeStorage<F, P, A, B>, // c0
    c1_source: ArcGenericLdeStorage<F, P, A, B>, // c1
    interpolation_degree_log2: usize,            // 折叠维度
    roots_precomputation: &[F],                  // 预计算的 twiddles
    challenges: Vec<(F, F)>,                     // 2-to-1 的折叠因子
    original_lde_degree: usize,                  // LDE 的 blowup factor
    coset_inverse: &mut F,                       // 1/g, where g is a primitive
    worker: &Worker, 
    _ctx: &mut P::Context,
) -> (Vec<F, A>, Vec<F, A>) {

    // 计算顶层多项式的 `full_size`
    let full_size = c0_source.outer_len() * c0_source.inner_len() * P::SIZE_FACTOR;
    // 检查 `roots_precomputation` 的长度是否正确
    debug_assert_eq!(roots_precomputation.len() * 2, full_size);

    // 把 2^k-degree 折叠拆解为 k 次数的 "对半折叠"
    let mut interpolation_degree_log2 = interpolation_degree_log2;
    let result_size = full_size >> 1;
    interpolation_degree_log2 -= 1;
    debug_assert!(result_size > 0);
    let mut result_c0 = Vec::with_capacity_in(result_size, A::default());
    let mut result_c1 = Vec::with_capacity_in(result_size, A::default());

    // we fold as many times as we need, but after first folding we should understand that our memory layout is not
    // beneficial for FRI, so in practice we work over independent field elements

    // 分解 challenges 为 (c0, c1)
    let (c0, c1) = challenges[0];

    // even though our cosets are continuous in memory, total placement is just bitreversed
    // 对矩阵的每一列进行折叠
    //   
    //    c0 = [ a_0, b_0, c_0, d_0]
    //         [ a_1, b_1, c_1, d_1]
    //         [ a_2, b_2, c_2, d_2]
    //         [ a_3, b_3, c_3, d_3]
    //
    //    result = | a'_0, b'_0, c'_0, d'_0 |
    //             | a'_1, b'_1, c'_1, d'_1 |

    for (coset_idx, (c0_coset, c1_coset)) in c0_source
        .storage
        .iter()
        .zip(c1_source.storage.iter())
        .enumerate()
    {
        let work_size = c0_coset.domain_size() / 2;
        let roots = &roots_precomputation[coset_idx * work_size..(coset_idx + 1) * work_size];
        let dst_c0 =
            &mut result_c0.spare_capacity_mut()[coset_idx * work_size..(coset_idx + 1) * work_size];
        let dst_c1 =
            &mut result_c1.spare_capacity_mut()[coset_idx * work_size..(coset_idx + 1) * work_size];

        worker.scope(work_size, |scope, chunk_size| {
            let src_c0 = P::slice_into_base_slice(&c0_coset.storage);
            let src_c1 = P::slice_into_base_slice(&c1_coset.storage);
            debug_assert_eq!(dst_c0.len() * 2, src_c0.len());

            for ((((c0_chunk, c1_chunk), dst0_chunk), dst1_chunk), roots) in src_c0
                .chunks(chunk_size * 2)
                .zip(src_c1.chunks(chunk_size * 2))
                .zip(dst_c0.chunks_mut(chunk_size))
                .zip(dst_c1.chunks_mut(chunk_size))
                .zip(roots.chunks(chunk_size))
            {
                scope.spawn(|_| {
                    fold_multiple::<F, EXT>(
                        c0_chunk,
                        c1_chunk,
                        dst0_chunk,
                        dst1_chunk,
                        roots,
                        &*coset_inverse,
                        (c0, c1),
                    );
                })
            }
        });
    }

    unsafe {
        result_c0.set_len(result_size);
        result_c1.set_len(result_size);
    }

    // 检查折叠后的多项式的最高次数是否等于 `full_size/original_lde_degree`
    coset_inverse.square();

    if crate::config::DEBUG_SATISFIABLE == false {
        let coset = coset_inverse.inverse().unwrap();
        let mut tmp: Vec<F, A> = result_c0.clone();
        bitreverse_enumeration_inplace(&mut tmp);
        crate::field::traits::field_like::ifft_natural_to_natural(&mut tmp, coset);
        for el in tmp[(tmp.len() / original_lde_degree)..].iter() {
            debug_assert_eq!(*el, F::ZERO);
        }

        let mut tmp = result_c1.clone();
        bitreverse_enumeration_inplace(&mut tmp);
        crate::field::traits::field_like::ifft_natural_to_natural(&mut tmp, coset);
        for el in tmp[(tmp.len() / original_lde_degree)..].iter() {
            debug_assert_eq!(*el, F::ZERO);
        }
    }

    // 取剩下的 challenges 数组，调用 interpolate_flattened_cosets() 继续剩下的 k-1 次折叠
    let challenges = challenges[1..].to_vec();

    interpolate_flattened_cosets::<F, EXT, A>(
        result_c0,
        result_c1,
        interpolation_degree_log2,
        roots_precomputation,
        challenges,
        original_lde_degree,
        coset_inverse,
        worker,
    )
}
```

下面我们看看 `interpolate_flattened_cosets()` 函数的实现

```rust
fn interpolate_flattened_cosets<
    F: SmallField,
    EXT: FieldExtension<2, BaseField = F>,
    A: GoodAllocator,
>(
    c0_source: Vec<F, A>,   // 原始多项式
    c1_source: Vec<F, A>,   // 原始多项式
    interpolation_degree_log2: usize, // 折叠维度
    roots_precomputation: &[F],       // 预计算的 roots
    challenges: Vec<(F, F)>,
    original_lde_degree: usize,
    coset_inverse: &mut F,
    worker: &Worker,
) -> (Vec<F, A>, Vec<F, A>) {
    let full_size = c0_source.len();
    debug_assert_eq!(interpolation_degree_log2, challenges.len());
    let max_result_size = full_size >> 1;
    debug_assert!(max_result_size > 0);

    let mut c0_source = c0_source;
    let mut c1_source = c1_source;

    // 定义折叠后的向量
    let mut result_c0 = Vec::with_capacity_in(max_result_size, A::default());
    let mut result_c1 = Vec::with_capacity_in(max_result_size, A::default());

    // 进行多次 "对半折叠"
    for (c0, c1) in challenges.into_iter() {
        let work_size = c0_source.len() / 2;

        // 清理 result 空间。因为每次折叠后，都会把 result 内容写回到 source 中
        result_c0.clear();
        result_c1.clear();

        // 取 roots 的前一半，
        // 比如 roots = [0, 8, 4, 12, 2, 10, 6, 14, 1, 9, 5, 13, 3, 11, 7, 15]
        //   前一半 roots_half = [0, 8, 4, 12, 2, 10, 6, 14]，恰好只留下偶数项
        //   如果再取前一半 [0,8,4,12] 恰好是 4 的整数倍
        let roots = &roots_precomputation[0..work_size];

        // 设置 dst 数组为 result_c0, result_c1 的 coset 切分
        let dst_c0 = &mut result_c0.spare_capacity_mut()[..work_size];
        let dst_c1 = &mut result_c1.spare_capacity_mut()[..work_size];

        // 并行折叠 
        // TODO: 地图
        worker.scope(work_size, |scope, chunk_size| {
            debug_assert_eq!(dst_c0.len() * 2, c0_source.len());

            for ((((c0_chunk, c1_chunk), dst0_chunk), dst1_chunk), roots) in c0_source
                .chunks(chunk_size * 2)
                .zip(c1_source.chunks(chunk_size * 2))
                .zip(dst_c0.chunks_mut(chunk_size))
                .zip(dst_c1.chunks_mut(chunk_size))
                .zip(roots.chunks(chunk_size))
            {
                scope.spawn(|_| {
                    fold_multiple::<F, EXT>(
                        c0_chunk,
                        c1_chunk,
                        dst0_chunk,
                        dst1_chunk,
                        roots,
                        &*coset_inverse,
                        (c0, c1),
                    );
                })
            }
        });

        // 设置 result_c0, result_c1 的长度
        unsafe {
            result_c0.set_len(work_size);
            result_c1.set_len(work_size);
        }

        // 检查折叠后的多项式的最高次数是否等于 `full_size/original_lde_degree`
        coset_inverse.square();

        if crate::config::DEBUG_SATISFIABLE == false {
            let coset = coset_inverse.inverse().unwrap();

            let mut tmp = result_c0.clone();
            bitreverse_enumeration_inplace(&mut tmp);
            crate::field::traits::field_like::ifft_natural_to_natural(&mut tmp, coset);
            for el in tmp[(tmp.len() / original_lde_degree)..].iter() {
                debug_assert_eq!(*el, F::ZERO);
            }

            let mut tmp = result_c1.clone();
            bitreverse_enumeration_inplace(&mut tmp);
            crate::field::traits::field_like::ifft_natural_to_natural(&mut tmp, coset);
            for el in tmp[(tmp.len() / original_lde_degree)..].iter() {
                debug_assert_eq!(*el, F::ZERO);
            }
        }

        // 交换 source 与 result，以便重用 buffer
        // swap source and result to reuse the buffer

        std::mem::swap(&mut c0_source, &mut result_c0);
        std::mem::swap(&mut c1_source, &mut result_c1);
    }

    (c0_source, c1_source)
}
```

最后一个相关函数 `fold_multiple()` 其实现如下：

```rust
fn fold_multiple<F: SmallField, EXT: FieldExtension<2, BaseField = F>>(
    c0_chunk: &[F],
    c1_chunk: &[F],
    dst_c0: &mut [MaybeUninit<F>],
    dst_c1: &mut [MaybeUninit<F>],
    roots: &[F],
    coset_inverse: &F,
    (c0, c1): (F, F),
) {
    // we compute f(x) + f(-x) + alpha * ((f(x) - f(-x))) / x,
    // where f(x), f(-x) and alpha are extension field elements,
    // and x is in the base field

    // So in practice we only do single multiplication of Fp2 by Fp2 here

    // 按照 32 个元素为一组进行折叠，折叠到 16 个元素
    let mut src_c0_chunks = c0_chunk.array_chunks::<32>();
    let mut src_c1_chunks = c1_chunk.array_chunks::<32>();

    let mut dst_c0_chunks = dst_c0.array_chunks_mut::<16>();
    let mut dst_c1_chunks = dst_c1.array_chunks_mut::<16>();

    let mut roots_chunks = roots.array_chunks::<16>();

    // 组装 challenge 为 ExtensionField
    let challenge_as_extension = ExtensionField::<F, 2, EXT> {
        coeffs: [c0, c1],
        _marker: std::marker::PhantomData,
    };

    // 并行折叠计算
    for ((((c0_pairs, c1_pairs), dst_c0), dst_c1), roots) in (&mut src_c0_chunks)
        .zip(&mut src_c1_chunks)
        .zip(&mut dst_c0_chunks)
        .zip(&mut dst_c1_chunks)
        .zip(&mut roots_chunks)
    {
        for i in 0..16 {

            // 计算 f0 = [c0(x) - c0(-x)] / x
            //  where x is precomputed roots (omegas)
            let f_at_x_c0 = c0_pairs[2 * i];
            let f_at_minus_x_c0 = c0_pairs[2 * i + 1];
            let mut diff_c0 = f_at_x_c0;
            diff_c0.sub_assign(&f_at_minus_x_c0);
            diff_c0.mul_assign(&roots[i]);
            diff_c0.mul_assign(coset_inverse);

            // 计算 f1 = [c1(x) - c1(-x)] / x
            let f_at_x_c1 = c1_pairs[2 * i];
            let f_at_minus_x_c1 = c1_pairs[2 * i + 1];
            let mut diff_c1 = f_at_x_c1;
            diff_c1.sub_assign(&f_at_minus_x_c1);
            diff_c1.mul_assign(&roots[i]);
            diff_c1.mul_assign(coset_inverse);

            // now we multiply
            let mut diff_as_extension = ExtensionField::<F, 2, EXT> {
                coeffs: [diff_c0, diff_c1],
                _marker: std::marker::PhantomData,
            };

            // 计算 [f0, f1] = alpha * [(c0(x) - c0(-x)) / x, (c1(x) - c1(-x)) / x]
            // 
            diff_as_extension.mul_assign(&challenge_as_extension);

            let [mut other_c0, mut other_c1] = diff_as_extension.into_coeffs_in_base();

            // 计算 f0 = c0(x) + c0(-x) + f0
            // 计算 f1 = c1(x) + c1(-x) + f1 
            other_c0.add_assign(&f_at_x_c0).add_assign(&f_at_minus_x_c0);
            other_c1.add_assign(&f_at_x_c1).add_assign(&f_at_minus_x_c1);

            dst_c0[i].write(other_c0);
            dst_c1[i].write(other_c1);
        }
    }

    // 计算最后一组（不足32个）的折叠
    // and now over remainders
    let c0_pairs = src_c0_chunks.remainder();
    let c1_pairs = src_c1_chunks.remainder();

    let dst_c0 = dst_c0_chunks.into_remainder();
    let dst_c1 = dst_c1_chunks.into_remainder();

    let roots = roots_chunks.remainder();

    let bound = dst_c0.len();

    for i in 0..bound {
        let f_at_x_c0 = c0_pairs[2 * i];
        let f_at_minus_x_c0 = c0_pairs[2 * i + 1];
        let mut diff_c0 = f_at_x_c0;
        diff_c0.sub_assign(&f_at_minus_x_c0);
        diff_c0.mul_assign(&roots[i]);
        diff_c0.mul_assign(coset_inverse);

        let f_at_x_c1 = c1_pairs[2 * i];
        let f_at_minus_x_c1 = c1_pairs[2 * i + 1];
        let mut diff_c1 = f_at_x_c1;
        diff_c1.sub_assign(&f_at_minus_x_c1);
        diff_c1.mul_assign(&roots[i]);
        diff_c1.mul_assign(coset_inverse);

        // now we multiply
        let mut diff_as_extension = ExtensionField::<F, 2, EXT> {
            coeffs: [diff_c0, diff_c1],
            _marker: std::marker::PhantomData,
        };

        diff_as_extension.mul_assign(&challenge_as_extension);

        let [mut other_c0, mut other_c1] = diff_as_extension.into_coeffs_in_base();

        other_c0.add_assign(&f_at_x_c0).add_assign(&f_at_minus_x_c0);
        other_c1.add_assign(&f_at_x_c1).add_assign(&f_at_minus_x_c1);

        dst_c0[i].write(other_c0);
        dst_c1[i].write(other_c1);
    }
}
```

### FRI Query 算法
