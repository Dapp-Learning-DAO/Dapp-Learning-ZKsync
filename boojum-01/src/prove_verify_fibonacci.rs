#![feature(allocator_api)]
use std::{
    alloc::Global,
    io::{Read, Write},
};

use boojum::{
    algebraic_props::{round_function::AbsorptionModeOverwrite, sponge::GoldilocksPoseidonSponge},
    config::DevCSConfig,
    cs::{
        cs_builder::{new_builder, CsBuilder, CsBuilderImpl},
        cs_builder_reference::CsReferenceImplementationBuilder,
        cs_builder_verifier::CsVerifierBuilder,
        gates::{
            ConstantAllocatableCS, ConstantsAllocatorGate, FmaGateInBaseFieldWithoutConstant,
            FmaGateInBaseWithoutConstantParams, NopGate,
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
fn prove_verify_fibonacci() {
    type P = GoldilocksField;

    // 和simple_fibonacci相同
    let geometry = CSGeometry {
        num_columns_under_copy_permutation: 8,
        num_witness_columns: 0,
        num_constant_columns: 2,
        max_allowed_constraint_degree: 8,
    };

    let max_variables = 512;
    let max_trace_len = 128;

    fn configure<
        F: SmallField,
        T: CsBuilderImpl<F, T>,
        GC: GateConfigurationHolder<F>,
        TB: StaticToolboxHolder,
    >(
        builder: CsBuilder<T, F, GC, TB>,
    ) -> CsBuilder<T, F, impl GateConfigurationHolder<F>, impl StaticToolboxHolder> {
        let builder = ConstantsAllocatorGate::configure_builder(
            builder,
            GatePlacementStrategy::UseGeneralPurposeColumns,
        );
        let builder = FmaGateInBaseFieldWithoutConstant::configure_builder(
            builder,
            GatePlacementStrategy::UseGeneralPurposeColumns,
        );
        let builder =
            NopGate::configure_builder(builder, GatePlacementStrategy::UseGeneralPurposeColumns);

        builder
    }

    // ---------------------------- verifier执行的部分 ----------------------------

    let verifier_builder_impl =
        CsReferenceImplementationBuilder::<GoldilocksField, P, DevCSConfig>::new(
            geometry.clone(),
            max_trace_len.clone(),
        );
    let verifier_builder = new_builder::<_, GoldilocksField>(verifier_builder_impl);

    let verifier_builder = configure(verifier_builder);
    let mut verifier_cs = verifier_builder.build(CircuitResolverOpts::new(max_variables));

    let mut previous_b = None;
    let mut previous_c = None;
    // 设置一个常量 1
    let one = verifier_cs.allocate_constant(GoldilocksField::ONE);

    // 证明第n个fibonacci数为out
    let n = 9;
    let out = 34;

    for _ in 0..n - 2 {
        let a = if let Some(previous) = previous_b {
            previous
        } else {
            // witness可以随便填，因为这里不需要验证，但是不能不填，否则会panic
            verifier_cs.alloc_single_variable_from_witness(GoldilocksField::ZERO)
        };
        let b = if let Some(previous) = previous_c {
            previous
        } else {
            verifier_cs.alloc_single_variable_from_witness(GoldilocksField::ZERO)
        };

        // c = a + b
        let c: Variable = FmaGateInBaseFieldWithoutConstant::compute_fma(
            &mut verifier_cs,
            GoldilocksField::ONE,
            (a, one),
            GoldilocksField::ONE,
            b,
        );
        previous_b = Some(b);
        previous_c = Some(c);
    }

    if let Some(c) = previous_c {
        let out = verifier_cs.allocate_constant(GoldilocksField::from_u64_unchecked(out));

        let gate = FmaGateInBaseFieldWithoutConstant {
            params: FmaGateInBaseWithoutConstantParams {
                coeff_for_quadtaric_part: GoldilocksField::ONE,
                linear_term_coeff: GoldilocksField::ZERO,
            },
            quadratic_part: (c, one),
            linear_part: one,
            rhs_part: out,
        };

        gate.add_to_cs(&mut verifier_cs);
    }

    // optional
    verifier_cs.pad_and_shrink();

    let worker = Worker::new_with_num_threads(1);
    let verifier_cs = verifier_cs.into_assembly::<Global>();

    let lde_factor_to_use = 16;
    let proof_config = ProofConfig {
        fri_lde_factor: lde_factor_to_use,
        pow_bits: 0,
        merkle_tree_cap_size: 4,
        ..Default::default()
    };

    // 得到verification key
    let (_, _, vk, _, _, _) = verifier_cs
        .get_full_setup::<GoldilocksPoseidonSponge<AbsorptionModeOverwrite>>(
            &worker,
            proof_config.fri_lde_factor,
            proof_config.merkle_tree_cap_size,
        );

    let vk_str = serde_json::to_string(&vk).expect("不能序列化verification key");
    let mut vk_file = std::fs::File::create("fibonacci_vk.json").expect("不能打开vk文件");
    vk_file
        .write_all(vk_str.as_bytes())
        .expect("不能写入vk文件");

    drop(vk);

    // ---------------------------- prover执行的部分 ----------------------------

    let prover_builder_impl =
        CsReferenceImplementationBuilder::<GoldilocksField, P, DevCSConfig>::new(
            geometry.clone(),
            max_trace_len.clone(),
        );
    let prover_builder = new_builder::<_, GoldilocksField>(prover_builder_impl);

    let prover_builder = configure(prover_builder);
    let mut prover_cs = prover_builder.build(CircuitResolverOpts::new(max_variables));

    let mut previous_b = None;
    let mut previous_c = None;
    let one = prover_cs.allocate_constant(GoldilocksField::ONE);

    // 证明第n个fibonacci数为out
    let n = 9;
    let out = 34;

    for _ in 0..n - 2 {
        let a = if let Some(previous) = previous_b {
            previous
        } else {
            prover_cs.alloc_single_variable_from_witness(GoldilocksField::ONE)
        };
        let b = if let Some(previous) = previous_c {
            previous
        } else {
            prover_cs.alloc_single_variable_from_witness(GoldilocksField::ONE)
        };

        // c = a + b
        let c: Variable = FmaGateInBaseFieldWithoutConstant::compute_fma(
            &mut prover_cs,
            GoldilocksField::ONE,
            (a, one),
            GoldilocksField::ONE,
            b,
        );
        previous_b = Some(b);
        previous_c = Some(c);
    }

    if let Some(c) = previous_c {
        let out = prover_cs.allocate_constant(GoldilocksField::from_u64_unchecked(out));
        //     cs.alloc_witness_without_value();
        // let out = Place::from_witness(out).as_variable();

        let gate = FmaGateInBaseFieldWithoutConstant {
            params: FmaGateInBaseWithoutConstantParams {
                coeff_for_quadtaric_part: GoldilocksField::ONE,
                linear_term_coeff: GoldilocksField::ZERO,
            },
            quadratic_part: (c, one),
            linear_part: one,
            rhs_part: out,
        };

        gate.add_to_cs(&mut prover_cs);
    }

    // optional
    prover_cs.pad_and_shrink();

    let prover_cs = prover_cs.into_assembly::<Global>();

    // 生成proof
    let (proof, _) = prover_cs.prove_one_shot::<
        GoldilocksExt2,
        GoldilocksPoisedonTranscript,
        GoldilocksPoseidonSponge<AbsorptionModeOverwrite>,
        NoPow,
    >(&worker, proof_config, ());

    let proof_str = serde_json::to_string(&proof).expect("不能序列化proof");
    let mut proof_file = std::fs::File::create("fibonacci_proof.json").expect("不能创建proof文件");
    proof_file
        .write_all(proof_str.as_bytes())
        .expect("不能写入proof文件");

    drop(proof);

    // ---------------------------- verifier执行的部分 ----------------------------

    let builder_impl =
        CsVerifierBuilder::<GoldilocksField, GoldilocksExt2>::new_from_parameters(geometry);
    let builder = new_builder::<_, GoldilocksField>(builder_impl);

    let builder = configure(builder);
    let verifier = builder.build(());

    let mut vk_file = std::fs::File::open("fibonacci_vk.json").expect("不能打开vk文件");
    let mut vk_str = "".to_string();
    vk_file.read_to_string(&mut vk_str).expect("不能读取vk文件");
    let vk = serde_json::from_str(&vk_str).expect("不能反序列化vk");

    let mut proof_file = std::fs::File::open("fibonacci_proof.json").expect("不能打开proof文件");
    let mut proof_str = "".to_string();
    proof_file
        .read_to_string(&mut proof_str)
        .expect("不能读取proof文件");
    let proof = serde_json::from_str(&proof_str).expect("不能反序列化proof");

    // 验证proof
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
