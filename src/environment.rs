use std::collections::HashMap;

use log::info;

use crate::core::SoxObject;
use crate::exceptions::RuntimeException;
use crate::token::Token;

#[derive(Clone, Debug)]
pub struct Namespace {
    pub bindings: HashMap<String, SoxObject>,
}

impl Default for Namespace {
    fn default() -> Self {
        let bindings = HashMap::new();
        Self { bindings }
    }
}

impl Namespace {
    pub fn define<T: Into<String>>(
        &mut self,
        name: T,
        value: SoxObject,
    ) -> Result<(), RuntimeException> {
        self.bindings.insert(name.into(), value);
        Ok(())
    }

    pub fn assign<T: Into<String>>(
        &mut self,
        name: T,
        value: SoxObject,
    ) -> Result<(), RuntimeException> {
        self.bindings.insert(name.into(), value);
        Ok(())
    }

    pub fn get<T: Into<String>>(&mut self, name: T) -> Result<SoxObject, RuntimeException> {
        let ret_val = if let Some(v) = self.bindings.get(name) {
            Ok(v.clone())
        } else {
            Err(RuntimeException {
                msg: "".into(),
            })
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
    pub fn define(&mut self, name: String, value: SoxObject) {
        let _ = self.namespaces.last_mut().unwrap().assign(name, value);
    }

    pub fn get(&mut self, name: &Token) -> Result<SoxObject, RuntimeException> {
        for namespace in self.namespaces.iter_mut().rev() {
            if let Ok(value) = namespace.get(name.lexeme.as_str()) {
                return Ok(value.clone());
            }
        }

        return Err(RuntimeException {
            msg: format!("Undefined variable {}.", name.lexeme),
        });
    }

    pub fn get_at(&mut self, p0: usize, p1: String) -> Result<SoxObject, RuntimeException> {
        let idx = self.namespaces.len() - 1 - p0;
        let namespace = self.namespaces.get_mut(idx);
        return if let Some(namespace) = namespace {
            let obj = namespace.get(p1.clone());
            if let Ok(obj) = obj {
                Ok(obj.clone())
            } else {
                info!("The namespace is {:?}", self.namespaces);
                Err(RuntimeException {
                    msg: format!("Variable[{:?}] not found in namespace", p1.clone()),
                })
            }
        } else {
            Err(RuntimeException {
                msg: "No namespace at index".into(),
            })
        };
    }

    pub fn assign(&mut self, name: Token, value: SoxObject) -> Result<SoxObject, RuntimeException> {
        for namespace in self.namespaces.iter_mut().rev() {
            if let Ok(_) = namespace.get(name.lexeme.as_str()) {
                namespace.assign(name.lexeme.as_str(), value.clone())?;
                return Ok(value);
            }
        }

        return Err(RuntimeException {
            msg: format!("Variable {} not defined in curr env.", name.lexeme),
        });
    }

    pub fn push(&mut self, namespace: Namespace) -> Result<(), RuntimeException> {
        self.namespaces.push(namespace);
        Ok(())
    }

    pub fn pop(&mut self) -> Result<(), RuntimeException> {
        self.namespaces.pop();
        Ok(())
    }
}

