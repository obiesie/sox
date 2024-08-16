use std::fmt::Display;

use crate::builtins::exceptions::{Exception, RuntimeError};
use crate::core::{SoxObject, SoxObjectPayload, SoxResult};

type EnvKey = (String, usize, usize);

#[derive(Clone, Debug)]
pub struct Namespace {
    pub bindings: Vec<(String, SoxObject)>,
}

impl Default for Namespace {
    fn default() -> Self {
        Namespace::new()
    }
}
impl Namespace {
    pub(crate) fn new() -> Self {
        let bindings = vec![];
        Self { bindings }
    }

    pub(crate) fn define<T: ToString + Display>(
        &mut self,
        key: T,
        value: SoxObject,
    ) -> SoxResult<()> {
        self.bindings.push((key.to_string(), value));
        Ok(())
    }

    fn assign(&mut self, key: &EnvKey, value: SoxObject) -> SoxResult<()> {
        let (name, _, binding_idx) = key;
        let mut entry = self.bindings.get_mut(*binding_idx);
        if entry.as_ref().unwrap().0 == *name {
            entry.as_mut().unwrap().1 = value;
        }
        Ok(())
    }

    fn get(&mut self, key: &EnvKey) -> SoxResult<SoxObject> {
        let (name, _, binding_idx) = key;
        //if let EnvKey::NameIdxPair((name, _, idx)) = key {
        let value = self.bindings.get(*binding_idx);
        let ret_val = if let Some(v) = value {
            Ok(v.1.clone())
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
        Env::new()
    }
}

impl Env {
    pub fn new() -> Self {
        Self {
            namespaces: vec![Namespace::new()],
        }
    }
    pub fn define<T: ToString + Display>(&mut self, key: T, value: SoxObject) {
        let ns = self.namespaces.last_mut().unwrap();
        let _ = ns.define(key, value);
    }

    pub fn get(&mut self, key: &EnvKey) -> SoxResult {
        let mut val = None;
        let mut name_literal = "".to_string();
        let (name, dist_to_ns, _) = key;
        name_literal = name.clone();
        let l = self.namespaces.len();
        let ns = self.namespaces.get_mut(l - dist_to_ns - 1).unwrap();
        val = Some(ns.get(key));

        if val.is_some() {
            return val.unwrap();
        }
        Err(Exception::Err(RuntimeError {
            msg: format!("NameError: name '{name_literal}' is not defined"),
        })
        .into_ref())
    }

    pub fn find_and_get<T: ToString + Display>(&mut self, key: T) -> SoxResult {
        let name_literal = key.to_string();
        for namespace in self.namespaces.iter_mut().rev() {
            let val = namespace
                .bindings
                .iter_mut()
                .find(|v| v.0 == key.to_string());
            if let Some(v) = val {
                return Ok(v.1.clone());
            }
        }
        Err(Exception::Err(RuntimeError {
            msg: format!("NameError: name '{name_literal}' is not defined"),
        })
        .into_ref())
    }

    pub fn find_and_assign<T: ToString + Display>(
        &mut self,
        key: T,
        value: SoxObject,
    ) -> SoxResult<()> {
        let name_literal = key.to_string();
        for namespace in self.namespaces.iter_mut().rev() {
            let val = namespace
                .bindings
                .iter_mut()
                .find(|v| v.0 == key.to_string());
            if let Some(v) = val {
                v.1 = value;
                return Ok(());
            }
        }
        Err(Exception::Err(RuntimeError {
            msg: format!("NameError: name '{name_literal}' is not defined"),
        })
        .into_ref())
    }

    pub fn assign(&mut self, key: &EnvKey, value: SoxObject) -> SoxResult<()> {
        let mut name_literal = "".to_string();
        let (name, dist_to_ns, _) = key;
        name_literal = name.clone();
        let l = self.namespaces.len();
        let ns = self.namespaces.get_mut(l - dist_to_ns - 1).unwrap();
        ns.assign(&key, value)?;
        Ok(())
    }

    pub fn new_namespace(&mut self) -> SoxResult<()> {
        let namespace = Namespace::new();
        let _ = self.namespaces.push(namespace);

        Ok(())
    }

    pub fn pop(&mut self) -> SoxResult<()> {
        self.namespaces.pop();
        Ok(())
    }

    pub fn push(&mut self, ns: Namespace) -> SoxResult<()> {
        self.namespaces.push(ns);
        Ok(())
    }

    pub fn size(&self) -> usize {
        self.namespaces.len()
    }
}
