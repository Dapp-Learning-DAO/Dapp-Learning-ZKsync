# Inside Boojum: FFT



## precompute_twiddles_for_fft

```rust
// N: 预计算 FFT 的 twiddles
pub fn precompute_twiddles_for_fft<
    F: PrimeField,
    P: field::traits::field_like::PrimeFieldLikeVectorized<Base = F>,
    A: GoodAllocator,
    const INVERSED: bool,
>(
    fft_size: usize,        // N: 需要 FFT 的多项式项数
    worker: &Worker,        // N: 线程池
    _ctx: &mut P::Context,  // N: 上下文
) -> Vec<P, A> {
    // N: 待 FFT 的多项式，长度必须补足为 2 的幂
    debug_assert!(fft_size.is_power_of_two());

    // N: omega ^ fft_size == 1
    let mut omega = domain_generator_for_size::<F>(fft_size as u64);
    // N: 如果作为 IFFT 算法，则 INVERSED 置为 TRUE，需要取 omega 的逆
    if INVERSED {
        omega = omega
            .inverse()
            .expect("must always exist for domain generator");
    }

    // N: 检查 fft_size 是 omega 的阶
    assert_eq!(omega.pow_u64(fft_size as u64), F::ONE);
    for i in 1..fft_size {
        assert_ne!(omega.pow_u64(i as u64), F::ONE);
    }

    // N: twiddle 的个数，即 num_powers，是待 FFT 的多项式长度(fft_size)的一半
    let num_powers = fft_size / 2;
    // MixedGL can have up to 16 elements, depending on implementation, so we can't
    // have less than that.
    // domain of size std::cmp::max(num_powers, P::SIZE_FACTOR) by generator omega
    let mut powers = materialize_powers_parallel::<F, P, A>(
        omega,
        std::cmp::max(num_powers, P::SIZE_FACTOR),
        worker,
    );

    // Items beyond `num_powers` are dead weight.
    bitreverse_enumeration_inplace(&mut powers[0..num_powers]);

    P::vec_from_base_vec(powers)
}
```

```rust
// N: 并行计算 powers of base
pub(crate) fn materialize_powers_parallel<
    F: PrimeField,
    P: field::traits::field_like::PrimeFieldLikeVectorized<Base = F>,
    A: GoodAllocator,
>(
    base: F,            // N: omega, omega ^ fft_size == 1
    size: usize,        // N: 返回的 powers 个数
    worker: &Worker,    // N: 线程池
) -> Vec<F, A> {
    if size == 0 {
        return Vec::new_in(A::default());
    }
    assert!(
        size.is_power_of_two(),
        "due to requirement on size and alignment we only allow powers of two sizes, but got {}",
        size
    );
    let size = std::cmp::max(size, P::SIZE_FACTOR);
    let mut storage = utils::allocate_in_with_alignment_of::<F, P, A>(size, A::default());
    worker.scope(size, |scope, chunk_size| {
        for (chunk_idx, chunk) in storage.spare_capacity_mut()[..size]
            // chunk_size = 8
            .chunks_mut(chunk_size)
            .enumerate()
        {
            // N: storage[chunk_idx * chunk_size + i]
            //    == base ^ (chunk_idx * chunk_size + i)
            //    == omega ^ (chunk_idx * chunk_size + i)
            //    <==>
            //    storage[i] == omega ^ i
            scope.spawn(move |_| {
                let mut current = base.pow_u64((chunk_idx * chunk_size) as u64);
                for el in chunk.iter_mut() {
                    el.write(current);
                    current.mul_assign(&base);
                }
            });
        }
    });

    unsafe { storage.set_len(size) }

    storage
}
```

```rust
// N: 计算 input 数组的 reverse bit order
// This operation is so cache-unfriendly, that parallelism is not used here
pub const fn bitreverse_enumeration_inplace<T>(input: &mut [T]) {
    if input.len() == 0 {
        return;
    }
    // N: 只有 2 的指数次方长度的数组才有 reverse bit order
    assert!(input.len().is_power_of_two());

    // N: SMALL_BITREVERSE_LOOKUP_TABLE 的大小为 2 ^ 6，MEDIUM_BITREVERSE_LOOKUP_TABLE 的大小为 2 ^ 8
    //    如果输入数组 input 长度小于 2 ^ 6，则使用 SMALL_BITREVERSE_LOOKUP_TABLE 查表排序
    //    如果输入数组 input 长度大于 2 ^ 6 且小于 2 ^ 8，则使用 MEDIUM_BITREVERSE_LOOKUP_TABLE 查表排序
    //    否则使用分段查表排序的方法（bitreverse_enumeration_inplace_hybrid）排序
    if input.len() <= SMALL_BITREVERSE_LOOKUP_TABLE.len() {
        bitreverse_enumeration_inplace_via_small_lookup(input);
    } else if input.len() <= MEDIUM_BITREVERSE_LOOKUP_TABLE.len() {
        bitreverse_enumeration_inplace_via_medium_lookup(input);
    } else {
        bitreverse_enumeration_inplace_hybrid(input);
    }
}
```

```rust
// N: 查表法得到 RBO 的数组下标，交换 input 中相应下标的元素
const fn bitreverse_enumeration_inplace_via_small_lookup<T>(input: &mut [T]) {
    // N: 只有 2 的指数次方长度的数组才有 reverse bit order
    assert!(input.len().is_power_of_two());
    // N: 只有长度小于 2 ^ 6 的数组才能使用 SMALL_BITREVERSE_LOOKUP_TABLE 查表 RBO 数组下标
    assert!(input.len() <= SMALL_BITREVERSE_LOOKUP_TABLE.len());

    // N: SMALL_BITREVERSE_LOOKUP_TABLE 中的元素为 6 bit 非负整数
    //    而 input 数组的下标范围是 0..input.len().trailing_zeros()
    //    因此计算 RBO 下标时需要去掉后 shift_to_cleanup 位
    //    shift_to_cleanup == 6 - input.len().trailing_zeros()
    let shift_to_cleanup =
        (SMALL_BITREVERSE_LOOKUP_TABLE_LOG_2_SIZE as u32) - input.len().trailing_zeros();

    // N: 设置一个指针 i 遍历 input 数组下标 0..work_size (work_size == input.len())
    let mut i = 0;
    let work_size = input.len();
    while i < work_size {
        // N: input 长度为 2 ^ 6 时，input[j] 需要与 input[i] 交换位置
        let mut j = SMALL_BITREVERSE_LOOKUP_TABLE[i] as usize;
        // N: 因为 input 数组长度 <= 2 ^ 6，所以 j 需要移除最后 shift_to_cleanup 位
        j >>= shift_to_cleanup; // if our table size is larger than work size
        // N: 只用交换 input[i], input[j] 一次，因此不妨只在 i < j 的时候交换
        //    i == j 的时候不用交换
        if i < j {
            // N: 原地交换 input[i] 和 input[j]
            unsafe { input.swap_unchecked(i, j) };
        }

        i += 1;
    }
}
```

```rust
// N: 将长度大于 2 ^ 8 的 input 数组，通过两层循环，按照 RBO 排序
const fn bitreverse_enumeration_inplace_hybrid<T>(input: &mut [T]) {
    // N: 只有 2 的指数次方长度的数组才有 reverse bit order
    assert!(input.len().is_power_of_two());
    // N: 只接受长度大于 2 ^ 8 且小于等于 2 ^ 31 的数组
    assert!(input.len() > MEDIUM_BITREVERSE_LOOKUP_TABLE.len());
    assert!(input.len() <= 1usize << 31); // a reasonable upper bound to use u32 internally

    // N: 下面这段注释解释了 RBO 数组和自然顺序数组中下标的对应关系
    //    即两者的下标互为 bitreversing

    // there is a function usize::reverse_bits(), but if one looks into the compiler then
    // will see that it's something like (sorry for C code)
    // ```
    //     uint32_t bit_reverse32(uint32_t x)
    // {
    //     x = (x >> 16) | (x << 16);
    //     x = ((x & 0xFF00FF00) >> 8) | ((x & 0x00FF00FF) << 8);
    //     x = ((x & 0xF0F0F0F0) >> 4) | ((x & 0x0F0F0F0F) << 4);
    //     x = ((x & 0xCCCCCCCC) >> 2) | ((x & 0x33333333) << 2);
    //     return ((x & 0xAAAAAAAA) >> 1) | ((x & 0x55555555) << 1);
    // }
    // ```

    // since we bitreverse a continuous set of indexes, we can save a little by
    // doing two loops, such that one bitreverses (naively) some common bits,
    // and one that bitreversed uncommon via lookup

    // N: input 元素下标的 bit 长度
    //    假设 log_n == 14
    let log_n = input.len().trailing_zeros();
    // N: 留下 MEDIUM_BITREVERSE_LOOKUP_TABLE_LOG_2_SIZE 部分交给第二层循环处理
    //    common_part_log_n == 14 - 8 == 6
    let common_part_log_n = log_n - (MEDIUM_BITREVERSE_LOOKUP_TABLE_LOG_2_SIZE as u32);

    // double loop. Note the swapping approach:
    // - lowest bits become highest bits and change every time
    // - highest bits change become lowest bits and change rarely
    // so our "i" counter is a counter over highest bits, and our source is in the form (i << 8) + j
    // and our dst is (reversed_j << common_part_log_n) + reversed_i
    // and since our source and destination are symmetrical we can formally swap them
    // and have our writes cache-friendly

    // N: 按照 RBO 数组下标的顺序遍历，反过来计算原数组下标，然后交换数组中对应元素
    let mut i = 0;
    // N: work_size == 1 << 6
    let work_size = 1u32 << common_part_log_n;
    while i < work_size {
        // bitreversing byte by byte
        // N: bytes 是 RBO 下标中的 common part
        //    i 的真实范围是 0..work_size，这里假设 i 的大小为 u32，因此分为 4 个 byte
        //    i = 0x00000000, 0x00000001, 0x00000002, ... , 0x0000003F
        //    i.swap_bytes() = 0x00000000, 0x01000000, 0x02000000, ... , 0x3F000000
        //    i.swap_bytes().to_le_bytes() = [0x00, 0x00, 0x00, 0x00], [0x00, 0x00, 0x00, 0x01], [0x00, 0x00, 0x00, 0x02], ... , [0x00, 0x00, 0x00, 0x3F]
        let mut bytes = i.swap_bytes().to_le_bytes();
        // N: RBO 下标的最低 byte (对应原数组下标的最高 byte) bytes[0] 留给第二层循环处理
        bytes[0] = 0;                                                   // N: 0
        bytes[1] = MEDIUM_BITREVERSE_LOOKUP_TABLE[bytes[1] as usize];   // N: 0
        bytes[2] = MEDIUM_BITREVERSE_LOOKUP_TABLE[bytes[2] as usize];   // N: 0
        bytes[3] = MEDIUM_BITREVERSE_LOOKUP_TABLE[bytes[3] as usize];   // N: 0, 0x80, 0x40, ... , 0xFC
        // N: reversed_i == bytes >> (32 - 6) == bytes >> 26
        //               == 0x00, 0x20, 0x10, ... , 0x3F
        //    即 reversed_i 只保留 bytes 最后 common_part_log_n 位
        let reversed_i = u32::from_le_bytes(bytes) >> (32 - common_part_log_n);

        debug_assert!(reversed_i == i.reverse_bits() >> (32 - common_part_log_n));

        // N: uncommon part
        let mut j = 0;
        while j < MEDIUM_BITREVERSE_LOOKUP_TABLE.len() {
            // N: reversed_j 对应原数组下标的最高 byte （即 RBO 数组下标的最低 byte）
            let reversed_j = MEDIUM_BITREVERSE_LOOKUP_TABLE[j];
            let dst = ((i as usize) << MEDIUM_BITREVERSE_LOOKUP_TABLE_LOG_2_SIZE) | j;
            let src = ((reversed_j as usize) << common_part_log_n) | (reversed_i as usize);
            if dst < src {
                unsafe { input.swap_unchecked(src, dst) };
            }

            j += 1;
        }

        i += 1;
    }
}
```

## ifft_natural_to_natural

```rust
// N: 计算 input 多项式在 coset 上的逆傅立叶变换，输入包括了预计算的 twiddles
pub fn ifft_natural_to_natural<F: BaseField>(input: &mut [F], coset: F, twiddles: &[F]) {
    debug_assert!(input.len().is_power_of_two());
    if input.len() > 16 {
        debug_assert!(input.len() == twiddles.len() * 2);
    }

    // N: input 长度的对数
    let log_n = input.len().trailing_zeros();

    // N: input 数论变换，随后转化为 RBO
    serial_ct_ntt_natural_to_bitreversed(input, log_n, twiddles);
    // N: 从 RBO 转换为自然顺序
    bitreverse_enumeration_inplace(input);

    if coset != F::ONE {
        let coset = coset.inverse().expect("inverse of coset must exist");
        // N: input 每个元素乘以 inversed coset
        //      因为原先的 input 在 coset 上
        //      转换到系数式后要除以 coset
        distribute_powers(input, coset);
    }

    // N: IFFT 还需要乘以 n_inv 系数
    if input.len() > 1 {
        let n_inv = F::from_u64_with_reduction(input.len() as u64)
            .inverse()
            .unwrap();
        let mut i = 0;
        let work_size = input.len();
        while i < work_size {
            input[i].mul_assign(&n_inv);
            i += 1;
        }
    }
}
```

```rust
// N: NTT 数论变换，从自然顺序到位逆序(RBO)转换
pub(crate) fn serial_ct_ntt_natural_to_bitreversed<F: BaseField>(
    a: &mut [F],                // N: 多项式
    log_n: u32,                 // N: 多项式对数长度
    omegas_bit_reversed: &[F],  // N: 预计算的 RBO twiddles
) {
    let n = a.len();
    // N: 多项式 degree 为 0 则直接返回
    if n == 1 {
        return;
    }

    if a.len() > 16 {
        debug_assert!(n == omegas_bit_reversed.len() * 2);
    }
    debug_assert!(n == (1 << log_n) as usize);

    // N: 自顶向下迭代 log(n) 轮，初始轮单独处理
    let mut pairs_per_group = n / 2;    // N: 每轮处理的元素对个数
    let mut num_groups = 1;             // N: 分为几组处理
    let mut distance = n / 2;           // N: 每对元素之间的距离

    // N: 初始轮
    {
        // special case for omega = 1
        debug_assert!(num_groups == 1);
        // N: 该组起始下标
        let idx_1 = 0;
        // N: 该组结束下标
        let idx_2 = pairs_per_group;

        // N: 遍历该组的元素
        let mut j = idx_1;

        while j < idx_2 {
            let u = a[j];
            let v = a[j + distance];

            // tmp = a[j]
            let mut tmp = u;
            // tmp = a[j] - a[j + distance]
            tmp.sub_assign(&v);

            // a[j + distance] = a[j] - a[j + distance]
            a[j + distance] = tmp;
            // a[j] = a[j] + a[j + distance]
            a[j].add_assign(&v);

            j += 1;
        }

        // N: 计算下一次迭代的参数
        pairs_per_group /= 2;
        num_groups *= 2;
        distance /= 2;
    }

    // N: 后续迭代
    while num_groups < n {
        debug_assert!(num_groups > 1);
        let mut k = 0;
        while k < num_groups {
            // N: 该组起始下标
            let idx_1 = k * pairs_per_group * 2;
            // N: 该组结束下标
            let idx_2 = idx_1 + pairs_per_group;
            let s = omegas_bit_reversed[k];

            // N: 遍历该组的元素
            let mut j = idx_1;
            while j < idx_2 {
                let u = a[j];
                let mut v = a[j + distance];
                v.mul_assign(&s);

                let mut tmp = u;
                tmp.sub_assign(&v);

                a[j + distance] = tmp;
                a[j].add_assign(&v);

                j += 1;
            }

            k += 1;
        }

        // N: 计算下一次迭代的参数
        pairs_per_group /= 2;
        num_groups *= 2;
        distance /= 2;
    }
}
```

```rust
// N: input 的各项乘以 powers of element
pub fn distribute_powers<F: PrimeField>(input: &mut [F], element: F) {
    let work_size = input.len();
    let mut scale_by = F::ONE;
    let mut idx = 0;
    while idx < work_size {
        input[idx].mul_assign(&scale_by);
        scale_by.mul_assign(&element);
        idx += 1;
    }
}
```

## transform_raw_storages_to_lde

```rust
// N: low degree extension
pub(crate) fn transform_raw_storages_to_lde<
    F: SmallField,
    P: field::traits::field_like::PrimeFieldLikeVectorized<Base = F>,
    A: GoodAllocator,
    B: GoodAllocator,
>(
    trace_columns: Vec<Vec<P, A>, B>,           // N: variables columns
    domain_size: usize,                         // N: 多项式所在 domain 大小
    lde_degree: usize,                          // N: 即 blow up factor
    inverse_twiddles: &P::InverseTwiddles<A>,   // N: twiddles 的倒数形式
    forward_twiddles: &P::Twiddles<A>,          // N: twiddles
    worker: &Worker,                            // N: 线程池
    ctx: &mut P::Context,                       // N: 上下文
) -> Vec<ArcGenericLdeStorage<F, P, A, B>, B> {
    assert!(lde_degree.is_power_of_two());
    assert!(lde_degree > 1);

    // N: trace_columns.len() == number of columns
    //    trace_columns[0].len() == domain_size / P::SIZE_FACTOR
    debug_assert_eq!(domain_size, trace_columns[0].len() * P::SIZE_FACTOR);

    let _num_polys = trace_columns.len();

    let _now = std::time::Instant::now();

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

    // N: FFT to get evaluation form
    transform_monomials_to_lde(jobs, domain_size, lde_degree, forward_twiddles, worker, ctx)
}
```

```rust
// N: 对一组多项式进行并行加速的 FFT
pub(crate) fn transform_monomials_to_lde<
    F: SmallField,
    P: field::traits::field_like::PrimeFieldLikeVectorized<Base = F>,
    A: GoodAllocator,
    B: GoodAllocator,
>(
    trace_columns: Vec<Vec<P, A>, B>,   // N: variables columns in coefficient form
    domain_size: usize,                 // N: 多项式所在 domain 大小
    lde_degree: usize,                  // N: 即 blow up factor
    forward_twiddles: &P::Twiddles<A>,  // N: 预计算 FFT twiddles
    worker: &Worker,                    // N: 线程池
    ctx: &mut P::Context,               // N: 上下文
) -> Vec<ArcGenericLdeStorage<F, P, A, B>, B> {
    assert!(lde_degree.is_power_of_two());
    assert!(lde_degree > 1);

    profile_fn!(transform_monomials_to_lde);
    profile_section!(determine_properties);
    let num_polys = trace_columns.len();

    debug_assert_eq!(domain_size, trace_columns[0].len() * P::SIZE_FACTOR);

    // N: 经过 lde 后的多项式 evaluation point 个数
    let lde_size = domain_size * lde_degree;
    // N: coset ^ lde_size == coset ^ (domain_size * lde_degree) == 1
    let coset = domain_generator_for_size::<F>(lde_size as u64);
    debug_assert!({
        let outer_omega = domain_generator_for_size::<F>(domain_size as u64);
        let tmp = coset.pow_u64(lde_degree as u64);

        outer_omega == tmp
    });

    let multiplicative_generator = F::multiplicative_generator();

    // now all polys are in the Monomial form, so let's LDE them
    // N: powers_of_coset = RBO([1, coset, coset^2, ... , coset^(lde_degree - 1)])
    let mut powers_of_coset = materialize_powers_serial::<F, A>(coset, lde_degree);
    bitreverse_enumeration_inplace(&mut powers_of_coset);
    let powers_of_coset_ref = &powers_of_coset[..];

    drop(determine_properties);
    profile_section!(extend_vecs);
    let _now = std::time::Instant::now();

    // N: jobs_per_coset == trace_columns.len() == variables 列数
    //      接下来按照 jobs_per_coset 个多项式一组分组，
    //      每组在 powers_of_coset_ref[coset_idx] * multiplicative_generator 的 coset 上运算，
    //      并行 FFT
    let jobs_per_coset = trace_columns.len();
    // we will create a temporary placeholder of vectors for more even CPU load
    // N: all_ldes.len() == lde_degree
    let mut all_ldes = Vec::with_capacity_in(trace_columns.len() * lde_degree, B::default());
    for _ in 0..(lde_degree - 1) {
        all_ldes.extend_from_slice(&trace_columns);
    }
    all_ldes.extend(trace_columns);

    drop(extend_vecs);
    profile_section!(do_work);
    worker.scope(all_ldes.len(), |scope, chunk_size| {
        for (chunk_idx, chunk) in all_ldes.chunks_mut(chunk_size).enumerate() {
            let mut ctx = *ctx;
            // N: 并行计算
            scope.spawn(move |_| {
                for (idx, poly) in chunk.iter_mut().enumerate() {
                    // N: 总共有 lde_degree 组，每组 trace_columns.len() 个，共 trace_columns.len() * lde_degree 个多项式
                    //    poly_idx 标记了当前多项式在所有多项式中的位次
                    let poly_idx = chunk_idx * chunk_size + idx;
                    // N: jobs_per_coset 为一组，找到当前多项式所在的组 coset_idx = poly_idx / jobs_per_coset (jobs_per_coset == trace_columns.len())
                    let coset_idx = poly_idx / jobs_per_coset;
                    // N: 找到当前组对应的 coset factor = powers_of_coset_ref[coset_idx]
                    let mut coset = powers_of_coset_ref[coset_idx];
                    // N: 非测试环境中 coset 还需要乘以 multiplicative_generator
                    if crate::config::DEBUG_SATISFIABLE == false {
                        coset.mul_assign(&multiplicative_generator);
                    }
                    debug_assert!(poly.as_ptr().addr() % std::mem::align_of::<P>() == 0);
                    // N: 对该多项式进行 FFT
                    P::fft_natural_to_bitreversed(poly, coset, forward_twiddles, &mut ctx);
                }
            });
        }
    });

    drop(do_work);
    profile_section!(finalize);
    // transpose them back. In "all_ldes" we have first coset for every poly, then next one for every, etc
    // and we need to place them into individual cosets for into poly

    // N: 待返回的数据结构
    let mut columns = Vec::with_capacity_in(num_polys, B::default());
    columns.resize(
        num_polys,
        ArcGenericLdeStorage::empty_with_capacity_in(lde_degree, B::default()),
    );

    // N: 重新排序，将同一个多项式的所有 lde 放入同一个 ArcGenericLdeStorage 数据结构
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

```rust
// N: input 数组从自然顺序到 RBO 顺序的 FFT，输入包括 coset 和预计算 FFT twiddles
pub fn fft_natural_to_bitreversed<F: BaseField>(input: &mut [F], coset: F, twiddles: &[F]) {
    debug_assert!(input.len().is_power_of_two());
    if input.len() > 16 {
        debug_assert!(input.len() == twiddles.len() * 2);
    }

    // N: 求多项式在 coset 上的值，因此需要分配 powers of coset
    if coset != F::ONE {
        distribute_powers(input, coset);
    }

    let log_n = input.len().trailing_zeros();

    // N: 调用 NTT 算法
    serial_ct_ntt_natural_to_bitreversed(input, log_n, twiddles);
}
```
