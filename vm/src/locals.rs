// Copyright (c) zkMove Authors
// SPDX-License-Identifier: Apache-2.0

use crate::value::Value;
use error::{RuntimeError, StatusCode, VmResult};
use halo2_proofs::arithmetic::FieldExt;
use std::{cell::RefCell, rc::Rc};

pub struct Locals<F: FieldExt>(Rc<RefCell<Vec<Value<F>>>>);

impl<F: FieldExt> Locals<F> {
    pub fn new(size: usize) -> Self {
        Self(Rc::new(RefCell::new(vec![Value::Invalid; size])))
    }

    pub fn copy(&self, index: usize) -> VmResult<Value<F>> {
        let values = self.0.borrow();
        match values.get(index) {
            Some(Value::Invalid) => Err(RuntimeError::new(StatusCode::CopyLocalError)),
            Some(v) => Ok(v.clone()),
            None => Err(RuntimeError::new(StatusCode::OutOfBounds)),
        }
    }

    pub fn store(&mut self, index: usize, value: Value<F>) -> VmResult<()> {
        let mut values = self.0.borrow_mut();
        match values.get_mut(index) {
            // Todo: check ref count
            Some(_v) => {
                values[index] = value;
                Ok(())
            }
            None => Err(RuntimeError::new(StatusCode::OutOfBounds)),
        }
    }

    pub fn move_(&self, index: usize) -> VmResult<Value<F>> {
        let mut values = self.0.borrow_mut();
        match values.get_mut(index) {
            Some(Value::Invalid) => Err(RuntimeError::new(StatusCode::MoveLocalError)),
            Some(v) => Ok(std::mem::replace(v, Value::Invalid)),
            None => Err(RuntimeError::new(StatusCode::OutOfBounds)),
        }
    }
}

impl<F: FieldExt> Locals<F> {
    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: usize) -> Option<Value<F>> {
        let values = self.0.borrow();
        values.get(index).cloned()
    }
}

impl<F: FieldExt> Clone for Locals<F> {
    fn clone(&self) -> Self {
        let inner = (0..self.len()).map(|i| self.get(i).unwrap()).collect();
        Locals(Rc::new(RefCell::new(inner)))
    }
}

impl<F: FieldExt> std::fmt::Debug for Locals<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        for i in 0..self.len() {
            writeln!(f, "locals[{}]:{:?}", i, self.get(i))?;
        }
        Ok(())
    }
}
