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
            ConstantAllocatableCS, ConstantsAllocatorGate, FmaGateInBaseFieldWithoutConstant,
            NopGate, PublicInputGate, ReductionGate, ReductionGateParams,
        },
        implementations::{
            pow::NoPow, prover::ProofConfig, transcript::GoldilocksPoisedonTranscript,
        },
        traits::{cs::ConstraintSystem, gate::GatePlacementStrategy},
        CSGeometry, GateConfigurationHolder, StaticToolboxHolder,
    },
    dag::CircuitResolverOpts,
    field::{
        goldilocks::{GoldilocksExt2, GoldilocksField},
        Field, SmallField, U64Representable,
    },
    worker::Worker,
};

#[test]
fn simple_poly() {
    type P = GoldilocksField;

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
