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
            BooleanConstraintGate, FmaGateInBaseFieldWithoutConstant,
            FmaGateInBaseWithoutConstantParams, NopGate,
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
        Field, SmallField,
    },
    worker::Worker,
};

#[test]
fn boolean_demo() {
    type P = GoldilocksField;

    // 设置电路参数
    let geometry = CSGeometry {
        num_columns_under_copy_permutation: 16,
        num_witness_columns: 0,
        num_constant_columns: 2,
        max_allowed_constraint_degree: 5,
    };

    let max_variables = 32; // variable数量上限
    let max_trace_len = 16; // 电路表格的行数上限

    // 配置cs的函数
    fn configure<
        F: SmallField,
        T: CsBuilderImpl<F, T>,
        GC: GateConfigurationHolder<F>,
        TB: StaticToolboxHolder,
    >(
        builder: CsBuilder<T, F, GC, TB>,
    ) -> CsBuilder<T, F, impl GateConfigurationHolder<F>, impl StaticToolboxHolder> {
        let builder = BooleanConstraintGate::configure_builder(
            builder,
            GatePlacementStrategy::UseGeneralPurposeColumns,
        );

        let builder = FmaGateInBaseFieldWithoutConstant::configure_builder(
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

    let b_true = BooleanConstraintGate::alloc_boolean_from_witness(&mut cs, true);
    let one = cs.alloc_single_variable_from_witness(GoldilocksField::ONE);

    let gate = FmaGateInBaseFieldWithoutConstant {
        params: FmaGateInBaseWithoutConstantParams {
            coeff_for_quadtaric_part: GoldilocksField::ZERO,
            linear_term_coeff: GoldilocksField::ONE,
        },
        quadratic_part: (b_true, b_true),
        linear_part: b_true,
        rhs_part: one,
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
        merkle_tree_cap_size: 1,
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
