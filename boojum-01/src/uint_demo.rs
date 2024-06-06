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
            FmaGateInBaseWithoutConstantParams, NopGate, ReductionGate, ReductionGateParams,
            UIntXAddGate, ZeroCheckGate,
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
    gadgets::{
        tables::{create_xor8_table, Xor8Table},
        traits::witnessable::CSWitnessable,
        u8::UInt8,
    },
    worker::Worker,
};

#[test]
fn uint8_demo() {
    type P = GoldilocksField;

    // 设置电路参数
    let geometry = CSGeometry {
        num_columns_under_copy_permutation: 16,
        num_witness_columns: 0,
        num_constant_columns: 4,
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
        let builder = ReductionGate::<_, 4>::configure_builder(
            builder,
            GatePlacementStrategy::UseGeneralPurposeColumns,
        );

        let builder = UIntXAddGate::<8>::configure_builder(
            builder,
            GatePlacementStrategy::UseGeneralPurposeColumns,
        );

        let builder = ConstantsAllocatorGate::configure_builder(
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

    assert!(cs.gate_is_allowed::<UIntXAddGate<8>>());
    assert!(cs.gate_is_allowed::<ConstantsAllocatorGate<_>>());
    assert!(cs.gate_is_allowed::<FmaGateInBaseFieldWithoutConstant<_>>());

    let one = cs.alloc_single_variable_from_witness(GoldilocksField::ONE);
    let one = unsafe { UInt8::<GoldilocksField>::from_variable_unchecked(one) };
    let result1 = one; // 1
    let result2 = one.add_no_overflow(&mut cs, one); // 2
    let result3 = one.sub_no_overflow(&mut cs, one); // 0
    let result4 = one.into_num().mul(&mut cs, &one.into_num()); // 1
    let result4 =
        unsafe { UInt8::<GoldilocksField>::from_variable_unchecked(result4.as_variables_set()[0]) };

    let four = cs.alloc_single_variable_from_witness(GoldilocksField::from_u64_unchecked(4));

    let gate = ReductionGate {
        params: ReductionGateParams {
            reduction_constants: [GoldilocksField::ONE; 4],
        },
        terms: [
            result1.as_variables_set()[0],
            result2.as_variables_set()[0],
            result3.as_variables_set()[0],
            result4.as_variables_set()[0],
        ],
        reduction_result: four,
    };

    gate.add_to_cs(&mut cs);

    // ReductionGate::reduce_terms(
    //     &mut cs,
    //     [
    //         GoldilocksField::ONE,
    //         GoldilocksField::ZERO,
    //         GoldilocksField::ZERO,
    //         GoldilocksField::ZERO,
    //     ],
    //     [v.as_variables_set()[0]; 4],
    // );

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
