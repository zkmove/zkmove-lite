// Copyright (c) zkMove Authors
// SPDX-License-Identifier: Apache-2.0

use crate::chips::evaluation_chip::NUM_OF_ADVICE_COLUMNS;
use crate::value::Value;
use crate::{assign_cond, assign_operands, div_rem};
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Chip, Layouter, Region},
    plonk::{Advice, Column, ConstraintSystem, Error, Selector},
    poly::Rotation,
};
use logger::prelude::*;
use movelang::value::{convert_to_field, move_div, move_rem, MoveValue};
use std::marker::PhantomData;

#[derive(Clone, Debug)]
pub struct ModConfig<F: FieldExt> {
    advices: [Column<Advice>; NUM_OF_ADVICE_COLUMNS],
    s_mod: Selector,
    _marker: PhantomData<F>,
}

pub struct ModChip<F: FieldExt> {
    config: ModConfig<F>,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> Chip<F> for ModChip<F> {
    type Config = ModConfig<F>;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}

impl<F: FieldExt> ModChip<F> {
    pub(crate) fn construct(
        config: <Self as Chip<F>>::Config,
        _loaded: <Self as Chip<F>>::Loaded,
    ) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    pub(crate) fn configure(
        meta: &mut ConstraintSystem<F>,
        advices: [Column<Advice>; NUM_OF_ADVICE_COLUMNS],
    ) -> <Self as Chip<F>>::Config {
        let s_mod = meta.selector();
        meta.create_gate("div_rem", |meta| {
            let lhs = meta.query_advice(advices[0], Rotation::cur());
            let rhs = meta.query_advice(advices[1], Rotation::cur());
            let quotient = meta.query_advice(advices[2], Rotation::cur());
            let remainder = meta.query_advice(advices[0], Rotation::next());
            let cond = meta.query_advice(advices[3], Rotation::cur());
            let s_mod = meta.query_selector(s_mod) * cond;

            vec![s_mod * (lhs - rhs * quotient - remainder)]
        });

        ModConfig {
            s_mod,
            advices,
            _marker: PhantomData,
        }
    }

    pub(crate) fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        a: Value<F>,
        b: Value<F>,
        cond: Option<F>,
    ) -> Result<Value<F>, Error> {
        let config = self.config();

        let mut c = None;
        layouter.assign_region(
            || "rem",
            |mut region: Region<'_, F>| {
                config.s_mod.enable(&mut region, 0)?;

                let (quotient, remainder) = div_rem!(a, b);
                assign_operands!(a, b, region, config);
                assign_cond!(cond, region, config);

                let _quotient_cell = region.assign_advice(
                    || "quotient",
                    config.advices[2],
                    0,
                    || quotient.ok_or(Error::Synthesis),
                )?;

                let remainder_cell = region.assign_advice(
                    || "remainder",
                    config.advices[0],
                    1,
                    || remainder.ok_or(Error::Synthesis),
                )?;
                c = Some(
                    Value::new_variable(remainder, Some(remainder_cell.cell()), a.ty())
                        .map_err(|_| Error::Synthesis)?,
                );
                Ok(())
            },
        )?;

        Ok(c.unwrap())
    }
}
