// Copyright (c) zkMove Authors
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::useless_conversion)]

use anyhow::Result;
use movelang::argument::ScriptArguments;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// directives can be added to move source files to tell vm how to run the test.
///
/// //! mods: arith.move - import a module
/// //! args: 0, 1       - pass arguments to the script, multiple args should separate with comma

#[derive(Debug)]
pub struct RunConfig {
    pub args: Option<ScriptArguments>,
    pub modules: Vec<String>,
}

impl RunConfig {
    pub fn new(script_file: &Path) -> Result<RunConfig> {
        let mut config = RunConfig {
            args: None,
            modules: vec![],
        };
        let file_str = script_file.to_str().expect("path is None.");

        let mut f = File::open(script_file)
            .map_err(|err| std::io::Error::new(err.kind(), format!("{}: {}", err, file_str)))?;
        let mut buffer = String::new();
        f.read_to_string(&mut buffer)?;

        for line in buffer.lines().into_iter() {
            let s = line.split_whitespace().collect::<String>();
            if let Some(s) = s.strip_prefix("//!args:") {
                config.args = Some(s.parse::<ScriptArguments>()?);
            }
            if let Some(s) = s.strip_prefix("//!mods:") {
                config.modules.push(s.to_string()); //todo: support multiple modules
            }
        }
        Ok(config)
    }
}
