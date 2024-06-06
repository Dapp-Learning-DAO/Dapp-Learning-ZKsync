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
            FmaGateInBaseWithoutConstantParams, NopGate, ReductionGate,
        },
        implementations::{
            lookup_table::LookupTable, pow::NoPow, prover::ProofConfig,
            transcript::GoldilocksPoisedonTranscript,
        },
        traits::{cs::ConstraintSystem, gate::GatePlacementStrategy},
        CSGeometry, GateConfigurationHolder, LookupParameters, StaticToolboxHolder,
    },
    dag::CircuitResolverOpts,
    field::{
        goldilocks::{GoldilocksExt2, GoldilocksField},
        Field, SmallField, U64Representable,
    },
    worker::Worker,
};
use derivative::Derivative;

#[test]
fn lookup_demo() {
    pub const TEST_TABLE_NAME: &str = "Test table";

    // 设置一个空的结构体，用来标识这个lookup table
    #[derive(Derivative)]
    #[derivative(Clone, Copy, Debug)]
    pub struct TestTableMarker;

    type P = GoldilocksField;

    // 填充lookup table
    fn create_test_table<F: SmallField>() -> LookupTable<F, 5> {
        let mut all_keys = Vec::with_capacity(64);
        for a in 0..8 {
            for b in 0..8 {
                let key = smallvec::smallvec![
                    F::from_u64_unchecked(a as u64),
                    F::from_u64_unchecked(b as u64)
                ];
                all_keys.push(key);
            }
        }
        LookupTable::new_from_keys_and_generation_function(
            &all_keys,
            TEST_TABLE_NAME.to_string(),
            2,
            |keys| {
                let a = keys[0].as_u64_reduced() as u8;
                let b = keys[1].as_u64_reduced() as u8;

                let xor_result = a ^ b;
                let or_result = a | b;
                let and_result = a & b;

                smallvec::smallvec![
                    F::from_u64_unchecked(xor_result as u64),
                    F::from_u64_unchecked(or_result as u64),
                    F::from_u64_unchecked(and_result as u64)
                ]
            },
        )
    }

    let geometry = CSGeometry {
        num_columns_under_copy_permutation: 8,
        num_witness_columns: 0,
        num_constant_columns: 2,
        max_allowed_constraint_degree: 8,
    };

    let max_variables = 1 << 16;
    let max_trace_len = 128;

    // allow lookup

    fn configure<
        F: SmallField,
        T: CsBuilderImpl<F, T>,
        GC: GateConfigurationHolder<F>,
        TB: StaticToolboxHolder,
    >(
        builder: CsBuilder<T, F, GC, TB>,
    ) -> CsBuilder<T, F, impl GateConfigurationHolder<F>, impl StaticToolboxHolder> {
        // 允许lookup
        let builder = builder.allow_lookup(
            LookupParameters::UseSpecializedColumnsWithTableIdAsConstant {
                width: 5,
                num_repetitions: 2,
                share_table_id: true,
            },
        );
        let builder = ConstantsAllocatorGate::configure_builder(
            builder,
            GatePlacementStrategy::UseGeneralPurposeColumns,
        );
        let builder = FmaGateInBaseFieldWithoutConstant::configure_builder(
            builder,
            GatePlacementStrategy::UseGeneralPurposeColumns,
        );
        // we pad with NOP gates, so we should formally allow it
        // 如果不使用pad_and_shrink，这里需要删去NopGate
        let builder =
            NopGate::configure_builder(builder, GatePlacementStrategy::UseGeneralPurposeColumns);

        builder
    }

    let builder_impl = CsReferenceImplementationBuilder::<GoldilocksField, P, DevCSConfig>::new(
        geometry,
        max_trace_len,
    );
    let builder = new_builder::<_, GoldilocksField>(builder_impl);

    let builder = configure(builder);
    let mut cs = builder.build(CircuitResolverOpts::new(max_variables));

    let table = create_test_table();
    let table_id = cs.add_lookup_table::<TestTableMarker, 5>(table);

    let one = cs.allocate_constant(GoldilocksField::ONE);
    let three = cs.alloc_single_variable_from_witness(GoldilocksField::from_u64_unchecked(3));

    for _i in 0..101 {
        let a = cs.alloc_single_variable_from_witness(GoldilocksField::from_u64_unchecked(1));
        let b = cs.alloc_single_variable_from_witness(GoldilocksField::from_u64_unchecked(2));

        // create some imbalance
        let [xor, _or, _and] = cs.perform_lookup(table_id, &[a, b]);

        let gate = FmaGateInBaseFieldWithoutConstant {
            params: FmaGateInBaseWithoutConstantParams {
                coeff_for_quadtaric_part: GoldilocksField::ONE,
                linear_term_coeff: GoldilocksField::ZERO,
            },
            quadratic_part: (xor, one),
            linear_part: one,
            rhs_part: three,
        };

        gate.add_to_cs(&mut cs);
    }

    // make few constants
    cs.allocate_constant(GoldilocksField::from_u64_unchecked(3));

    cs.pad_and_shrink();

    // NOTE: it's here only to check constant propagation
    let must_be_allowed = cs.gate_is_allowed::<ConstantsAllocatorGate<GoldilocksField>>();
    assert!(must_be_allowed);

    // NOTE: config中不能有未使用的gate
    let may_be_in_config = cs.gate_is_allowed::<ReductionGate<GoldilocksField, 4>>();
    assert!(may_be_in_config == false);

    // 8线程worker
    let worker = Worker::new_with_num_threads(8);

    let mut cs = cs.into_assembly::<Global>();

    assert!(cs.check_if_satisfied(&worker));

    let lde_factor_to_use = 16;
    let proof_config = ProofConfig {
        fri_lde_factor: lde_factor_to_use,
        pow_bits: 0,
        ..Default::default()
    };

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
