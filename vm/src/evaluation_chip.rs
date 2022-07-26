// Copyright (c) zkMove Authors
// SPDX-License-Identifier: Apache-2.0

use crate::chips::arithmetic::{ArithmeticChip, ArithmeticConfig};
use crate::chips::conditional_select::{ConditionalSelectChip, ConditionalSelectConfig};
use crate::chips::logical::{LogicalChip, LogicalConfig};
use crate::chips::utilities::NUM_OF_ADVICE_COLUMNS;
use crate::instructions::Opcode;
use crate::value::Value;
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Chip, Layouter},
    plonk::{Advice, Column, ConstraintSystem, Error, Fixed, Instance},
};
use movelang::value::MoveValueType;
use std::marker::PhantomData;

#[derive(Clone, Debug)]
pub struct EvaluationConfig<F: FieldExt> {
    advice: [Column<Advice>; NUM_OF_ADVICE_COLUMNS],
    instance: Column<Instance>, // Public inputs
    constant: Column<Fixed>,    // Fixed column to load constants
    arithmetic_config: ArithmeticConfig,
    logical_config: LogicalConfig<F>,
    conditional_select_config: ConditionalSelectConfig,
}

pub struct EvaluationChip<F: FieldExt> {
    config: EvaluationConfig<F>,
    arithmetic_chip: ArithmeticChip<F>,
    logical_chip: LogicalChip<F>,
    conditional_select_chip: ConditionalSelectChip<F>,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> Chip<F> for EvaluationChip<F> {
    type Config = EvaluationConfig<F>;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}

impl<F: FieldExt> EvaluationChip<F> {
    pub fn construct(
        config: <Self as Chip<F>>::Config,
        _loaded: <Self as Chip<F>>::Loaded,
    ) -> Self {
        let arithmetic_config = config.arithmetic_config.clone();
        let arithmetic_chip = ArithmeticChip::<F>::construct(arithmetic_config, ());

        let logical_config = config.logical_config.clone();
        let logical_chip = LogicalChip::<F>::construct(logical_config, ());

        let conditional_select_config = config.conditional_select_config.clone();
        let conditional_select_chip =
            ConditionalSelectChip::<F>::construct(conditional_select_config, ());

        Self {
            config,
            arithmetic_chip,
            logical_chip,
            conditional_select_chip,
            _marker: PhantomData,
        }
    }

    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        advice: [Column<Advice>; NUM_OF_ADVICE_COLUMNS],
        instance: Column<Instance>,
        constant: Column<Fixed>,
    ) -> <Self as Chip<F>>::Config {
        let arithmetic_config = ArithmeticChip::configure(meta, advice);
        let logical_config = LogicalChip::configure(meta, advice);
        let conditional_select_config = ConditionalSelectChip::configure(meta, advice);

        meta.enable_equality(instance);
        meta.enable_constant(constant);

        EvaluationConfig {
            advice,
            instance,
            constant,
            arithmetic_config,
            logical_config,
            conditional_select_config,
            //other config
        }
    }

    pub fn conditional_select(
        &self,
        layouter: impl Layouter<F>,
        a: Value<F>,
        b: Value<F>,
        cond: Option<F>,
    ) -> Result<Value<F>, Error> {
        self.conditional_select_chip
            .conditional_select(layouter, a, b, cond)
    }

    pub fn binary_op(
        &self,
        layouter: impl Layouter<F>,
        opcode: Opcode,
        a: Value<F>,
        b: Value<F>,
        cond: Option<F>,
    ) -> Result<Value<F>, Error> {
        match opcode {
            Opcode::Add => self.arithmetic_chip.add(layouter, a, b, cond),
            Opcode::Sub => self.arithmetic_chip.sub(layouter, a, b, cond),
            Opcode::Mul => self.arithmetic_chip.mul(layouter, a, b, cond),
            Opcode::Div => self.arithmetic_chip.div(layouter, a, b, cond),
            Opcode::Mod => self.arithmetic_chip.rem(layouter, a, b, cond),
            Opcode::Eq => self.logical_chip.eq(layouter, a, b, cond),
            Opcode::Neq => self.logical_chip.neq(layouter, a, b, cond),
            Opcode::And => self.logical_chip.and(layouter, a, b, cond),
            Opcode::Or => self.logical_chip.or(layouter, a, b, cond),
            Opcode::Lt => self.logical_chip.lt(layouter, a, b, cond),
            _ => unreachable!(),
        }
    }

    pub fn unary_op(
        &self,
        layouter: impl Layouter<F>,
        opcode: Opcode,
        a: Value<F>,
        cond: Option<F>,
    ) -> Result<Value<F>, Error> {
        match opcode {
            Opcode::Not => self.logical_chip.not(layouter, a, cond),
            _ => unreachable!(),
        }
    }

    pub fn load_private(
        &self,
        mut layouter: impl Layouter<F>,
        value: Option<F>,
        ty: MoveValueType,
    ) -> Result<Value<F>, Error> {
        let config = self.config();

        let mut alloc = None;
        layouter.assign_region(
            || "load private",
            |mut region| {
                let cell = region.assign_advice(
                    || "private input",
                    config.advice[0],
                    0,
                    || value.ok_or(Error::Synthesis),
                )?;
                alloc = Some(
                    Value::new_variable(value, Some(cell.cell()), ty.clone())
                        .map_err(|_| Error::Synthesis)?,
                );
                Ok(())
            },
        )?;
        Ok(alloc.unwrap())
    }

    pub fn load_constant(
        &self,
        mut layouter: impl Layouter<F>,
        constant: F,
        ty: MoveValueType,
    ) -> Result<Value<F>, Error> {
        let config = self.config();

        let mut alloc = None;
        layouter.assign_region(
            || "load constant",
            |mut region| {
                let cell = region.assign_fixed(
                    || "constant value",
                    config.constant,
                    0,
                    || Ok(constant),
                )?;
                alloc = Some(
                    Value::new_constant(constant, Some(cell.cell()), ty.clone())
                        .map_err(|_| Error::Synthesis)?,
                );

                Ok(())
            },
        )?;
        Ok(alloc.unwrap())
    }

    pub fn expose_public(
        &self,
        mut layouter: impl Layouter<F>,
        value: Value<F>,
        row: usize,
    ) -> Result<(), Error> {
        let config = self.config();

        layouter.constrain_instance(value.cell().unwrap(), config.instance, row)
    }
}
