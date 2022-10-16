// Copyright (c) zkMove Authors
// SPDX-License-Identifier: Apache-2.0

use crate::chips::evaluation_chip::{EvaluationChip, EvaluationConfig, NUM_OF_ADVICE_COLUMNS};
use crate::chips::instructions::Opcode;
use crate::chips::utilities::{
    RangeCheckChip, RangeCheckConfig, NUM_OF_BYTES_U128, NUM_OF_BYTES_U64, NUM_OF_BYTES_U8,
};
use crate::value::Value;
use halo2_proofs::poly::Rotation;
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Layouter, Region, SimpleFloorPlanner},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Expression, Selector},
};
use logger::prelude::*;
use movelang::value::MoveValueType;

struct TestCircuit<F: FieldExt> {
    a: Option<F>,
    a_type: MoveValueType,
    b: Option<F>,
    b_type: MoveValueType,
    cond: Option<F>,
}

impl<F: FieldExt> Circuit<F> for TestCircuit<F> {
    type Config = EvaluationConfig<F>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self {
            a: None,
            a_type: MoveValueType::U8,
            b: None,
            b_type: MoveValueType::U8,
            cond: None,
        }
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let advice = [
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
        ];
        let instance = meta.instance_column();
        let constant = meta.fixed_column();

        EvaluationChip::configure(meta, advice, instance, constant)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let evaluation_chip = EvaluationChip::<F>::construct(config, ());

        let a = evaluation_chip.load_private(
            layouter.namespace(|| "load a"),
            self.a,
            self.a_type.clone(),
        )?;
        let b = evaluation_chip.load_private(
            layouter.namespace(|| "load b"),
            self.b,
            self.b_type.clone(),
        )?;
        let c = evaluation_chip.binary_op(
            layouter.namespace(|| "a + b"),
            Opcode::Add,
            a.clone(),
            b.clone(),
            self.cond,
        )?;
        let d = evaluation_chip.binary_op(
            layouter.namespace(|| "a - b"),
            Opcode::Sub,
            a.clone(),
            b.clone(),
            self.cond,
        )?;
        let e = evaluation_chip.binary_op(
            layouter.namespace(|| "a * b"),
            Opcode::Mul,
            a.clone(),
            b.clone(),
            self.cond,
        )?;

        let f = evaluation_chip.binary_op(
            layouter.namespace(|| "a == b"),
            Opcode::Eq,
            a,
            b,
            self.cond,
        )?;

        evaluation_chip.expose_public(layouter.namespace(|| "expose c"), c, 0)?;
        evaluation_chip.expose_public(layouter.namespace(|| "expose d"), d, 1)?;
        evaluation_chip.expose_public(layouter.namespace(|| "expose e"), e, 2)?;
        evaluation_chip.expose_public(layouter.namespace(|| "expose f"), f, 3)?;
        Ok(())
    }
}

#[derive(Clone)]
struct TestBranchCircuit<F: FieldExt> {
    a: Option<F>,
    a_type: MoveValueType,
    b: Option<F>,
    b_type: MoveValueType,
    cond: Option<F>,
}

impl<F: FieldExt> Circuit<F> for TestBranchCircuit<F> {
    type Config = EvaluationConfig<F>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self {
            a: None,
            a_type: MoveValueType::U8,
            b: None,
            b_type: MoveValueType::U8,
            cond: None,
        }
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let advice = [
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
        ];
        let instance = meta.instance_column();
        let constant = meta.fixed_column();

        EvaluationChip::configure(meta, advice, instance, constant)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let evaluation_chip = EvaluationChip::<F>::construct(config, ());

        let a = evaluation_chip.load_private(
            layouter.namespace(|| "load a"),
            self.a,
            self.a_type.clone(),
        )?;
        let b = evaluation_chip.load_private(
            layouter.namespace(|| "load b"),
            self.b,
            self.b_type.clone(),
        )?;
        let not_cond = self.cond.map(|v| F::one() - v);
        let c = evaluation_chip.binary_op(
            layouter.namespace(|| "a + b"),
            Opcode::Add,
            a.clone(),
            b.clone(),
            self.cond,
        )?;
        let d = evaluation_chip.binary_op(
            layouter.namespace(|| "a * b"),
            Opcode::Mul,
            a,
            b,
            not_cond,
        )?;

        let out = evaluation_chip.conditional_select(
            layouter.namespace(|| "conditional select"),
            c,
            d,
            self.cond,
        )?;
        evaluation_chip.expose_public(layouter.namespace(|| "expose out"), out, 0)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct RangeCheckTestConfig<F: FieldExt> {
    advices: [Column<Advice>; NUM_OF_ADVICE_COLUMNS],
    s_a: Selector,
    range_check_u8: RangeCheckConfig<F, NUM_OF_BYTES_U8>,
    range_check_u64: RangeCheckConfig<F, NUM_OF_BYTES_U64>,
    range_check_u128: RangeCheckConfig<F, NUM_OF_BYTES_U128>,
}

struct RangeCheckTestCircuit<F: FieldExt> {
    a: Value<F>,
    cond: F,
}

impl<F: FieldExt> Circuit<F> for RangeCheckTestCircuit<F> {
    type Config = RangeCheckTestConfig<F>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self {
            a: Value::u8(0, None).unwrap(),
            cond: F::zero(),
        }
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let advices = [
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
        ];
        for column in &advices {
            meta.enable_equality(*column);
        }

        let s_a = meta.selector();
        meta.create_gate("a", |meta| {
            let s_a = meta.query_selector(s_a);
            meta.query_advice(advices[0], Rotation::cur());
            vec![s_a * Expression::Constant(F::zero())]
        });

        let range_check_u8 = RangeCheckChip::configure(meta, advices);
        let range_check_u64 = RangeCheckChip::configure(meta, advices);
        let range_check_u128 = RangeCheckChip::configure(meta, advices);

        RangeCheckTestConfig {
            advices,
            s_a,
            range_check_u8,
            range_check_u64,
            range_check_u128,
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let mut value = None;
        layouter.assign_region(
            || "range check",
            |mut region: Region<'_, F>| {
                config.s_a.enable(&mut region, 0)?;
                let a = region.assign_advice(
                    || "a",
                    config.advices[0],
                    0,
                    || {
                        self.a.value().ok_or_else(|| {
                            error!("a.value() is None");
                            Error::Synthesis
                        })
                    },
                )?;
                value = Some(
                    Value::new_variable(self.a.value(), Some(a.cell()), self.a.ty())
                        .map_err(|_| Error::Synthesis)?,
                );

                Ok(())
            },
        )?;

        match self.a.ty() {
            MoveValueType::U8 => {
                RangeCheckChip::construct(config.range_check_u8).assign(
                    &mut layouter,
                    value.unwrap(),
                    Some(self.cond),
                )?;
            }
            MoveValueType::U64 => {
                RangeCheckChip::construct(config.range_check_u64).assign(
                    &mut layouter,
                    value.unwrap(),
                    Some(self.cond),
                )?;
            }
            MoveValueType::U128 => {
                RangeCheckChip::construct(config.range_check_u128).assign(
                    &mut layouter,
                    value.unwrap(),
                    Some(self.cond),
                )?;
            }
            _ => unimplemented!(),
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::chips::chip_tests::TestCircuit;
    use crate::chips::chip_tests::{RangeCheckTestCircuit, TestBranchCircuit};
    use crate::value::Value;
    use halo2_proofs::dev::MockProver;
    use halo2_proofs::pasta::{EqAffine, Fp};
    use halo2_proofs::plonk::{create_proof, keygen_pk, keygen_vk, verify_proof, SingleVerifier};
    use halo2_proofs::poly::commitment::Params;
    use halo2_proofs::transcript::{Blake2bRead, Blake2bWrite, Challenge255};
    use movelang::value::MoveValueType;
    use rand_core::OsRng;

    #[test]
    fn test_evaluation() {
        // Circuit is very small, we pick a small value here
        let k = 5;

        // Prepare the private and public inputs to the circuit
        let a = Fp::from(3);
        let b = Fp::from(2);
        let c = a + b;
        let d = a - b;
        let e = a * b;
        let f = Fp::zero();
        let cond = Fp::one();

        // Instantiate the circuit with the private inputs
        let circuit = TestCircuit {
            a: Some(a),
            a_type: MoveValueType::U8,
            b: Some(b),
            b_type: MoveValueType::U8,
            cond: Some(cond),
        };

        let mut public_inputs = vec![c, d, e, f];

        // Given the correct public input, circuit will verify
        let prover = MockProver::run(k, &circuit, vec![public_inputs.clone()]).unwrap();
        assert_eq!(prover.verify(), Ok(()));

        // If use some other public input, the proof will fail
        public_inputs[1] = Fp::zero();
        let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
        assert!(prover.verify().is_err());
    }

    #[test]
    fn test_branch() {
        // Circuit is very small, we pick a small value here
        let k = 5;
        let params: Params<EqAffine> = Params::new(k);

        let empty_circuit = TestBranchCircuit {
            a: None,
            a_type: MoveValueType::U8,
            b: None,
            b_type: MoveValueType::U8,
            cond: None,
        };

        let vk = keygen_vk(&params, &empty_circuit).expect("keygen_vk should not fail");
        let pk = keygen_pk(&params, vk, &empty_circuit).expect("keygen_pk should not fail");

        // Prepare the private and public inputs to the circuit
        let a = Fp::from(2);
        let b = Fp::from(3);
        let c = a + b;
        let d = a * b;
        let cond = Fp::one();

        // Instantiate the circuit with the private inputs
        let circuit = TestBranchCircuit {
            a: Some(a),
            a_type: MoveValueType::U8,
            b: Some(b),
            b_type: MoveValueType::U8,
            cond: Some(cond),
        };
        let public_inputs = vec![c];

        // Given the correct public input, circuit will verify
        let prover = MockProver::run(k, &circuit, vec![public_inputs.clone()]).unwrap();
        assert_eq!(prover.verify(), Ok(()));

        // If use some other public input, the proof will fail
        let wrong_public_inputs = vec![d];
        let prover = MockProver::run(k, &circuit, vec![wrong_public_inputs]).unwrap();
        assert!(prover.verify().is_err());

        let mut transcript = Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);
        // Create a proof
        create_proof(
            &params,
            &pk,
            &[circuit],
            &[&[public_inputs.as_slice()]],
            OsRng,
            &mut transcript,
        )
        .expect("proof generation should not fail");
        let proof: Vec<u8> = transcript.finalize();

        let strategy = SingleVerifier::new(&params);

        let mut transcript = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);
        let result = verify_proof(
            &params,
            pk.get_vk(),
            strategy,
            &[&[public_inputs.as_slice()]],
            &mut transcript,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_range_check_1() {
        // Circuit is very small, we pick a small value here
        let k = 5;

        // Prepare the private and public inputs to the circuit
        let a = Fp::from(2);
        let b = Fp::from(3);
        let c = a + b;
        let d = a - b; // -1 is out of range
        let e = a * b;
        let f = Fp::zero();
        let cond = Fp::one();

        // Instantiate the circuit with the private inputs
        let circuit = TestCircuit {
            a: Some(a),
            a_type: MoveValueType::U8,
            b: Some(b),
            b_type: MoveValueType::U8,
            cond: Some(cond),
        };

        let public_inputs = vec![c, d, e, f];

        let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
        // should fail
        assert!(prover.verify().is_err());
    }

    #[test]
    fn test_range_check_2() {
        // Circuit is very small, we pick a small value here
        let k = 5;

        // Prepare the private and public inputs to the circuit
        let a = Fp::from(255);
        let b = Fp::from(1);
        let c = a + b; // 256 is out of range
        let d = a - b;
        let e = a * b;
        let f = Fp::zero();
        let cond = Fp::one();

        // Instantiate the circuit with the private inputs
        let circuit = TestCircuit {
            a: Some(a),
            a_type: MoveValueType::U8,
            b: Some(b),
            b_type: MoveValueType::U8,
            cond: Some(cond),
        };

        let public_inputs = vec![c, d, e, f];

        let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
        // should fail
        assert!(prover.verify().is_err());
    }

    #[test]
    fn test_range_check() {
        let k = 5;
        let a = Value::u8(2, None).unwrap();
        let cond = Fp::from(1);
        let circuit = RangeCheckTestCircuit { a, cond };

        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()));

        let a = Value::u64(256, None).unwrap();
        let cond = Fp::from(1);
        let circuit = RangeCheckTestCircuit { a, cond };

        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()));

        let a = Value::u128(18446744073709551616, None).unwrap();
        let cond = Fp::from(1);
        let circuit = RangeCheckTestCircuit { a, cond };

        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }
}
