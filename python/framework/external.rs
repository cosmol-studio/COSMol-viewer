//! Typed references to separately installed Python extension classes.
//! Business modules declare their class once; no type registry is kept here.
use pyo3::BoundObject;
use pyo3::prelude::*;
use std::marker::PhantomData;

pub(crate) trait ExternalType {
    const MODULE: &'static str;
    const NAME: &'static str;
}

pub(crate) struct External<'py, T> {
    object: Bound<'py, PyAny>,
    marker: PhantomData<T>,
}

impl<'py, T> External<'py, T> {
    pub(crate) fn as_bound(&self) -> &Bound<'py, PyAny> {
        &self.object
    }
}

impl<'a, 'py, T: ExternalType> FromPyObject<'a, 'py> for External<'py, T> {
    type Error = PyErr;
    const INPUT_TYPE: pyo3::inspect::PyStaticExpr = pyo3::inspect::PyStaticExpr::Attribute {
        value: &pyo3::inspect::PyStaticExpr::Name { id: T::MODULE },
        attr: T::NAME,
    };

    fn extract(value: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        let class = value.py().import(T::MODULE)?.getattr(T::NAME)?;
        if !value.is_instance(&class)? {
            return Err(pyo3::exceptions::PyTypeError::new_err(format!(
                "Expected {}.{}",
                T::MODULE,
                T::NAME,
            )));
        }
        Ok(Self {
            object: value.to_owned().into_bound(),
            marker: PhantomData,
        })
    }
}
