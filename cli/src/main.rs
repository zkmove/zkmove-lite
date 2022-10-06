// Copyright (c) zkMove Authors
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::ptr_arg)]

use error::VmResult;
use functional_tests::run_config::RunConfig;
use halo2_proofs::pasta::{EqAffine, Fp};
use halo2_proofs::poly::commitment::Params;
use logger::prelude::*;
use movelang::argument::{parse_transaction_argument, ScriptArgument, ScriptArguments};
use movelang::compiler::compile_script;
use movelang::state::StateStore;
use std::path::PathBuf;
use std::process::exit;
use structopt::StructOpt;
use vm::runtime::Runtime;

#[derive(StructOpt)]
#[structopt(name = "zkmove", about = "CLI for zkMove Lite")]
pub struct Arguments {
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt)]
pub enum Command {
    #[structopt(
        name = "run",
        about = "Run the full sequence of circuit building, setup, proving, and verifying."
    )]
    Run {
        #[structopt(
            short = "s",
            long = "script-file",
            help = "path to .move file containing script"
        )]
        script: PathBuf,

        #[structopt(
            short = "m",
            long = "module-dir",
            help = "directory containing modules"
        )]
        modules: Option<PathBuf>,

        #[structopt(short = "d", long = "debug", help = "debug with mock prover")]
        use_mock: bool,

        #[structopt(
            long = "new-args",
            help = "run with new arguments, still use the old proving/verifying keys, multiple args should separate with space",
            parse(try_from_str = parse_transaction_argument)
        )]
        new_args: Option<Vec<ScriptArgument>>,

        #[structopt(short = "v", long = "verbose")]
        verbose: bool,

        #[structopt(long = "print-layout")]
        print_layout: bool,
    },
}

impl Arguments {
    pub fn run(
        &self,
        script: &PathBuf,
        module_dir: &Option<PathBuf>,
        use_mock: bool,
        new_args: &Option<Vec<ScriptArgument>>,
        verbose: bool,
        print_layout: bool,
    ) -> VmResult<()> {
        logger::init_for_main(verbose);

        let script_file = script.to_str().expect("path is None.");

        // compile script and depended modules
        let mut targets = vec![];
        targets.push(script_file.to_string());
        let config = RunConfig::new(script.as_path())?;
        for module in config.modules.into_iter() {
            let path = module_dir
                .clone()
                .expect("module_dir is missing")
                .as_path()
                .join(module)
                .to_str()
                .unwrap()
                .to_string();
            targets.push(path);
        }
        info!("compile script...");
        let (compiled_script, compiled_modules) = compile_script(targets)?;

        let script = compiled_script.expect("script is missing");
        let runtime = Runtime::<Fp>::new();
        let mut state = StateStore::new();
        for module in compiled_modules.clone().into_iter() {
            state.add_module(module);
        }

        let move_circuit = runtime.create_move_circuit(
            script.clone(),
            compiled_modules.clone(),
            config.args,
            state.clone(),
        );
        let public_inputs = vec![Fp::zero()];
        info!("find the best k...");
        let k = runtime.find_best_k(&move_circuit, vec![public_inputs.clone()])?;
        info!("k = {}", k);

        if use_mock {
            info!("run with mock prover...");
            runtime.mock_prove_circuit(&move_circuit, vec![public_inputs.clone()], k)?;
        }

        if print_layout {
            info!("print circuit layout into layout.svg ...");
            runtime.print_circuit_layout(k, &move_circuit);
        }

        info!("setup move circuit...");
        let params: Params<EqAffine> = Params::new(k);
        let pk = runtime.setup_move_circuit(&move_circuit, &params)?;

        info!("prove move circuit...");
        runtime.prove_move_circuit(
            move_circuit,
            &[public_inputs.as_slice()],
            &params,
            pk.clone(),
        )?;

        if let Some(new_args) = new_args {
            info!("execute script with new arguments");
            let arguments = Some(ScriptArguments::new(new_args.clone()));

            let new_move_circuit =
                runtime.create_move_circuit(script, compiled_modules, arguments, state);

            info!("prove the new execution with old proving key...");
            runtime.prove_move_circuit(
                new_move_circuit,
                &[public_inputs.as_slice()],
                &params,
                pk,
            )?;
        }

        Ok(())
    }
}

fn main() {
    let args = Arguments::from_args();

    let result = match args.cmd {
        Command::Run {
            ref script,
            ref modules,
            use_mock,
            ref new_args,
            verbose,
            print_layout,
        } => args.run(script, modules, use_mock, new_args, verbose, print_layout),
    };

    if let Err(error) = result {
        error!("{}", error);
        exit(1);
    }
}
