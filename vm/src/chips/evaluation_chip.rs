// Copyright (c) zkMove Authors
// SPDX-License-Identifier: Apache-2.0

use crate::chips::conditional_select::{ConditionalSelectChip, ConditionalSelectConfig};
use crate::chips::instructions::_mod::{ModChip, ModConfig};
use crate::chips::instructions::add::{AddChip, AddConfig};
use crate::chips::instructions::and::{AndChip, AndConfig};
use crate::chips::instructions::div::{DivChip, DivConfig};
use crate::chips::instructions::eq::{EqChip, EqConfig};
use crate::chips::instructions::lt::{LtChip, LtConfig};
use crate::chips::instructions::mul::{MulChip, MulConfig};
use crate::chips::instructions::neq::{NeqChip, NeqConfig};
use crate::chips::instructions::not::{NotChip, NotConfig};
use crate::chips::instructions::or::{OrChip, OrConfig};
use crate::chips::instructions::sub::{SubChip, SubConfig};
use crate::chips::instructions::Opcode;
use crate::chips::utilities::{
    RangeCheckChip, RangeCheckConfig, NUM_OF_BYTES_U128, NUM_OF_BYTES_U64, NUM_OF_BYTES_U8,
};
use crate::value::Value;
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Chip, Layouter},
    plonk::{Advice, Column, ConstraintSystem, Error, Fixed, Instance},
};
use movelang::value::MoveValueType;

pub const NUM_OF_ADVICE_COLUMNS: usize = 4;

#[derive(Clone, Debug)]
pub struct EvaluationConfig<F: FieldExt> {
    advices: [Column<Advice>; NUM_OF_ADVICE_COLUMNS],
    instance: Column<Instance>, // Public inputs
    constant: Column<Fixed>,    // Fixed column to load constants
    add_config: AddConfig<F>,
    sub_config: SubConfig<F>,
    mul_config: MulConfig<F>,
    div_config: DivConfig<F>,
    mod_config: ModConfig<F>,
    eq_config: EqConfig<F>,
    neq_config: NeqConfig<F>,
    and_config: AndConfig<F>,
    or_config: OrConfig<F>,
    not_config: NotConfig<F>,
    lt_config: LtConfig<F>,
    conditional_select_config: ConditionalSelectConfig,
    range_check_u8: RangeCheckConfig<F, NUM_OF_BYTES_U8>,
    range_check_u64: RangeCheckConfig<F, NUM_OF_BYTES_U64>,
    range_check_u128: RangeCheckConfig<F, NUM_OF_BYTES_U128>,
}

pub struct EvaluationChip<F: FieldExt> {
    config: EvaluationConfig<F>,
    conditional_select_chip: ConditionalSelectChip<F>,
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
        let conditional_select_config = config.conditional_select_config.clone();
        let conditional_select_chip =
            ConditionalSelectChip::<F>::construct(conditional_select_config, ());

        Self {
            config,
            conditional_select_chip,
        }
    }

    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        advices: [Column<Advice>; NUM_OF_ADVICE_COLUMNS],
        instance: Column<Instance>,
        constant: Column<Fixed>,
    ) -> <Self as Chip<F>>::Config {
        let add_config = AddChip::configure(meta, advices);
        let sub_config = SubChip::configure(meta, advices);
        let mul_config = MulChip::configure(meta, advices);
        let div_config = DivChip::configure(meta, advices);
        let mod_config = ModChip::configure(meta, advices);
        let eq_config = EqChip::configure(meta, advices);
        let neq_config = NeqChip::configure(meta, advices);
        let and_config = AndChip::configure(meta, advices);
        let or_config = OrChip::configure(meta, advices);
        let not_config = NotChip::configure(meta, advices);
        let lt_config = LtChip::configure(meta, advices);
        let conditional_select_config = ConditionalSelectChip::configure(meta, advices);
        let range_check_u8 = RangeCheckChip::configure(meta, advices);
        let range_check_u64 = RangeCheckChip::configure(meta, advices);
        let range_check_u128 = RangeCheckChip::configure(meta, advices);

        for column in &advices {
            meta.enable_equality(*column);
        }
        meta.enable_equality(instance);
        meta.enable_constant(constant);

        EvaluationConfig {
            advices,
            instance,
            constant,
            add_config,
            sub_config,
            mul_config,
            div_config,
            mod_config,
            eq_config,
            neq_config,
            and_config,
            or_config,
            not_config,
            lt_config,
            conditional_select_config,
            range_check_u8,
            range_check_u64,
            range_check_u128,
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

    fn range_check(
        &self,
        layouter: &mut impl Layouter<F>,
        value: Value<F>,
        cond: Option<F>,
    ) -> Result<(), Error> {
        match value.ty() {
            MoveValueType::U8 => {
                RangeCheckChip::construct(self.config.range_check_u8.clone())
                    .assign(layouter, value, cond)?;
            }
            MoveValueType::U64 => {
                RangeCheckChip::construct(self.config.range_check_u64.clone())
                    .assign(layouter, value, cond)?;
            }
            MoveValueType::U128 => {
                RangeCheckChip::construct(self.config.range_check_u128.clone())
                    .assign(layouter, value, cond)?;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn binary_op(
        &self,
        mut layouter: impl Layouter<F>,
        opcode: Opcode,
        a: Value<F>,
        b: Value<F>,
        cond: Option<F>,
    ) -> Result<Value<F>, Error> {
        let out = match opcode {
            Opcode::Add => {
                let add_chip = AddChip::<F>::construct(self.config.add_config.clone(), ());
                add_chip.assign(&mut layouter, a, b, cond)?
            }
            Opcode::Sub => {
                let sub_chip = SubChip::<F>::construct(self.config.sub_config.clone(), ());
                sub_chip.assign(&mut layouter, a, b, cond)?
            }
            Opcode::Mul => {
                let mul_chip = MulChip::<F>::construct(self.config.mul_config.clone(), ());
                mul_chip.assign(&mut layouter, a, b, cond)?
            }
            Opcode::Div => {
                let div_chip = DivChip::<F>::construct(self.config.div_config.clone(), ());
                div_chip.assign(&mut layouter, a, b, cond)?
            }
            Opcode::Mod => {
                let mod_chip = ModChip::<F>::construct(self.config.mod_config.clone(), ());
                mod_chip.assign(&mut layouter, a, b, cond)?
            }
            Opcode::Eq => {
                let eq_chip = EqChip::<F>::construct(self.config.eq_config.clone(), ());
                eq_chip.assign(&mut layouter, a, b, cond)?
            }
            Opcode::Neq => {
                let neq_chip = NeqChip::<F>::construct(self.config.neq_config.clone(), ());
                neq_chip.assign(&mut layouter, a, b, cond)?
            }
            Opcode::And => {
                let and_chip = AndChip::<F>::construct(self.config.and_config.clone(), ());
                and_chip.assign(&mut layouter, a, b, cond)?
            }
            Opcode::Or => {
                let or_chip = OrChip::<F>::construct(self.config.or_config.clone(), ());
                or_chip.assign(&mut layouter, a, b, cond)?
            }
            Opcode::Lt => {
                let lt_chip = LtChip::<F>::construct(self.config.lt_config.clone(), ());
                lt_chip.assign(&mut layouter, a, b, cond)?
            }
            _ => unreachable!(),
        };
        self.range_check(&mut layouter, out.clone(), cond)?;
        Ok(out)
    }

    pub fn unary_op(
        &self,
        mut layouter: impl Layouter<F>,
        opcode: Opcode,
        a: Value<F>,
        cond: Option<F>,
    ) -> Result<Value<F>, Error> {
        match opcode {
            Opcode::Not => {
                let not_chip = NotChip::<F>::construct(self.config.not_config.clone(), ());
                not_chip.assign(&mut layouter, a, cond)
            }
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
                    config.advices[0],
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
