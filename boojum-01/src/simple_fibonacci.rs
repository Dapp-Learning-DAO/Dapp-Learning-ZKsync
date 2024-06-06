#![feature(allocator_api)]
use std::alloc::Global;

use boojum::{
    algebraic_props::{round_function::AbsorptionModeOverwrite, sponge::GoldilocksPoseidonSponge},
    config::DevCSConfig,
    cs::{
        cs_builder::{new_builder, CsBuilder, CsBuilderImpl},
        cs_builder_reference::CsReferenceImplementationBuilder,
        cs_builder_verifier::CsVerifierBuilder,
        gates::{
            ConstantsAllocatorGate, FmaGateInBaseFieldWithoutConstant,
            FmaGateInBaseWithoutConstantParams, NopGate, PublicInputGate,
        },
        implementations::{
            pow::NoPow, prover::ProofConfig, transcript::GoldilocksPoisedonTranscript,
        },
        traits::{cs::ConstraintSystem, gate::GatePlacementStrategy},
        CSGeometry, GateConfigurationHolder, StaticToolboxHolder, Variable,
    },
    dag::CircuitResolverOpts,
    field::{
        goldilocks::{GoldilocksExt2, GoldilocksField},
        Field, SmallField, U64Representable,
    },
    worker::Worker,
};

#[test]
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
        // 在cs中加入 constant 门
        let builder = ConstantsAllocatorGate::configure_builder(
            builder,
            GatePlacementStrategy::UseGeneralPurposeColumns,
        );
        // 在cs中加入 public input 门
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
            cs.alloc_single_variable_from_witness(GoldilocksField::ONE)
        };
        let b = if let Some(previous) = previous_c {
            // b 为上一轮的 c
            previous
        } else {
            // 初始化 b 为 1
            cs.alloc_single_variable_from_witness(GoldilocksField::ONE)
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

    // optional
    cs.pad_and_shrink();

    // 设置线程数量
    let worker = Worker::new_with_num_threads(1);
    let cs = cs.into_assembly::<Global>();

    // FRI 的 LDE 因子
    let lde_factor_to_use = 16;
    let proof_config = ProofConfig {
        fri_lde_factor: lde_factor_to_use,
        pow_bits: 0,
        merkle_tree_cap_size: 4,
        ..Default::default()
    };

    // 使用prove_one_shot同时生成proof和vk，在生产环境不建议这样使用
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

    assert!(is_valid);
}
