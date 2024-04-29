use std::collections::HashMap;
use std::fmt::Display;

use log::{debug, info};

use crate::builtins::exceptions::{Exception, RuntimeError};
use crate::core::{SoxObject, SoxObjectPayload, SoxRef, SoxResult};
use crate::interpreter::Interpreter;

#[derive(Clone, Debug)]
pub struct Namespace {
    pub bindings: HashMap<String, SoxObject>,
}

impl Default for Namespace {
    fn default() -> Self {
        debug!("Creating new namespace in current environment");
        let bindings = HashMap::new();
        Self { bindings }
    }
}

impl Namespace {
    pub fn define<T: Into<String>>(&mut self, name: T, value: SoxObject) -> SoxResult<()> {
        self.bindings.insert(name.into(), value);
        Ok(())
    }

    pub fn assign<T: Into<String>>(&mut self, name: T, value: SoxObject) -> SoxResult<()> {
        self.bindings.insert(name.into(), value);
        Ok(())
    }

    pub fn get<T: AsRef<str> + Display>(&mut self, name: T) -> SoxResult<SoxObject> {
        let ret_val = if let Some(v) = self.bindings.get(name.as_ref()) {
            Ok(v.clone())
        } else {
            Err(Exception::Err(RuntimeError {
                msg: format!("NameError: name '{name}' is not defined"),
            })
            .into_ref())
        };
        ret_val
    }
}

#[derive(Clone, Debug)]
pub struct Env {
    pub namespaces: Vec<Namespace>,
}

impl Default for Env {
    fn default() -> Self {
        Self {
            namespaces: vec![Namespace::default()],
        }
    }
}

impl Env {
    pub fn define<T: Into<String>>(&mut self, name: T, value: SoxObject) {
        let _ = self.namespaces.last_mut().unwrap().define(name, value);
    }

    pub fn get<T: Into<String> + Display>(&mut self, name: T) -> SoxResult {
        let name_literal = name.into();
        for namespace in self.namespaces.iter_mut().rev() {
            if let Ok(value) = namespace.get(name_literal.as_str()) {
                return Ok(value.clone());
            }
        }
        info!("The environment is {:?}", self.namespaces);

        return Err(Exception::Err(RuntimeError {
            msg: format!("NameError: name '{name_literal}' is not defined"),
        })
        .into_ref());
    }

    pub fn assign<T: Into<String> + Display>(
        &mut self,
        name: T,
        value: SoxObject,
    ) -> SoxResult<()> {
        let name_literal = name.into();
        for namespace in self.namespaces.iter_mut().rev() {
            if let Ok(_) = namespace.get(name_literal.as_str()) {
                namespace.assign(name_literal, value)?;
                return Ok(());
            }
        }

        return Err(Exception::Err(RuntimeError {
            msg: format!("NameError: Name '{name_literal}' is not defined."),
        })
        .into_ref());
    }

    pub fn new_namespace(&mut self) -> SoxResult<()> {
        let namespace = Namespace::default();
        let _ = self.namespaces.push(namespace);

        Ok(())
    }

    pub fn pop(&mut self) -> SoxResult<()> {
        self.namespaces.pop();
        Ok(())
    }

    pub fn push(&mut self, namespace: Namespace) -> SoxResult<()> {
        self.namespaces.push(namespace);
        Ok(())
    }

    pub fn size(&self) -> usize {
        self.namespaces.len()
    }
}
