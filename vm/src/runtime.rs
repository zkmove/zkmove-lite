// Copyright (c) zkMove Authors
// SPDX-License-Identifier: Apache-2.0

use crate::circuit::MoveCircuit;
use error::{RuntimeError, StatusCode, VmResult};
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::{
    create_proof, keygen_pk, keygen_vk, verify_proof, Circuit, Error, ProvingKey, SingleVerifier,
};
use halo2_proofs::poly::commitment::Params;
use halo2_proofs::transcript::{Blake2bRead, Blake2bWrite, Challenge255};
use halo2_proofs::{dev::MockProver, pasta::EqAffine, pasta::Fp};
use logger::prelude::*;
use move_binary_format::file_format::CompiledScript;
use move_binary_format::CompiledModule;
use movelang::argument::ScriptArguments;
use movelang::loader::MoveLoader;
use movelang::state::StateStore;
use plotters::prelude::*;
use rand_core::OsRng;
use std::marker::PhantomData;

// number of circuit rows cannot exceed 2^MAX_K
pub const MAX_K: u32 = 18;
pub const MIN_K: u32 = 1;

pub struct Runtime<F: FieldExt> {
    loader: MoveLoader,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> Runtime<F> {
    pub fn new() -> Self {
        Runtime {
            loader: MoveLoader::new(),
            _marker: PhantomData,
        }
    }

    pub fn loader(&self) -> &MoveLoader {
        &self.loader
    }

    pub fn create_move_circuit(
        &self,
        script: CompiledScript,
        modules: Vec<CompiledModule>,
        args: Option<ScriptArguments>,
        data_store: StateStore,
    ) -> MoveCircuit {
        MoveCircuit::new(script, modules, args, data_store, self.loader())
    }

    // find the minimum k that satisfies the circuit row number less than 2^k
    pub fn find_best_k<ConcreteCircuit: Circuit<F>>(
        &self,
        circuit: &ConcreteCircuit,
        instance: Vec<Vec<F>>,
    ) -> VmResult<u32> {
        let mut k = MIN_K;
        while k <= MAX_K {
            trace!("Try k={}...", k);
            let not_enough_rows_error = Error::NotEnoughRowsAvailable { current_k: k };
            let result = MockProver::run(k, circuit, instance.clone());
            match result {
                Ok(_) => {
                    break;
                }
                Err(e) => {
                    if e.to_string() == not_enough_rows_error.to_string() {
                        k += 1;
                    } else {
                        debug!("Prover Error: {:?}", e);
                        return Err(RuntimeError::new(StatusCode::ProofSystemError(e)));
                    }
                }
            }
        }
        Ok(k)
    }

    pub fn mock_prove_circuit<ConcreteCircuit: Circuit<F>>(
        &self,
        circuit: &ConcreteCircuit,
        instance: Vec<Vec<F>>,
        k: u32,
    ) -> VmResult<()> {
        let prover = MockProver::run(k, circuit, instance).map_err(|e| {
            debug!("Prover Error: {:?}", e);
            RuntimeError::new(StatusCode::ProofSystemError(e))
        })?;
        assert_eq!(prover.verify(), Ok(()));

        Ok(())
    }

    pub fn print_circuit_layout<ConcreteCircuit: Circuit<F>>(
        &self,
        k: u32,
        circuit: &ConcreteCircuit,
    ) {
        let root = SVGBackend::new("layout.svg", (3840, 2160)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root.titled("Circuit Layout", ("sans-serif", 60)).unwrap();

        halo2_proofs::dev::CircuitLayout::default()
            .mark_equality_cells(true)
            .show_equality_constraints(true)
            .render(k, circuit, &root)
            .unwrap();
    }

    pub fn setup_move_circuit(
        &self,
        circuit: &MoveCircuit,
        params: &Params<EqAffine>,
    ) -> VmResult<ProvingKey<EqAffine>> {
        debug!("Generate vk");
        let vk = keygen_vk(params, circuit).map_err(|e| {
            RuntimeError::new(StatusCode::ProofSystemError(e))
                .with_message("keygen_vk should not fail".to_string())
        })?;
        debug!("Generate pk");
        let pk = keygen_pk(params, vk, circuit).map_err(|e| {
            RuntimeError::new(StatusCode::ProofSystemError(e))
                .with_message("keygen_pk should not fail".to_string())
        })?;
        Ok(pk)
    }

    pub fn prove_move_circuit(
        &self,
        circuit: MoveCircuit,
        instance: &[&[Fp]],
        params: &Params<EqAffine>,
        pk: ProvingKey<EqAffine>,
    ) -> VmResult<()> {
        let mut transcript = Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);
        // Create a proof
        let prove_start = std::time::Instant::now();
        create_proof(params, &pk, &[circuit], &[instance], OsRng, &mut transcript)
            .expect("proof generation should not fail");
        let proof: Vec<u8> = transcript.finalize();
        info!("proof size {} bytes", proof.len());
        let prove_time = std::time::Instant::now().duration_since(prove_start);
        info!("proving time: {} ms", prove_time.as_millis());

        let strategy = SingleVerifier::new(params);
        let mut transcript = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);
        let verify_start = std::time::Instant::now();
        let result = verify_proof(params, pk.get_vk(), strategy, &[instance], &mut transcript);
        let verify_time = std::time::Instant::now().duration_since(verify_start);
        info!("verification time: {} ms", verify_time.as_millis());
        assert!(result.is_ok());
        Ok(())
    }
}
