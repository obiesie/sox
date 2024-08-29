use std::collections::HashMap;

use log::{info};

use crate::builtins::exceptions::{Exception, RuntimeError};
use crate::core::{SoxObject, SoxObjectPayload, SoxResult};

#[derive(Clone, Debug)]
pub enum EnvKey {
    Name(String),
    NameIdxPair((String, usize, usize)),
}


#[derive(Clone, Debug)]
pub enum StoreMode {
    Vec,
    Map,
}
#[derive(Clone, Debug)]
pub struct Namespace {
    pub bindings: Vec<(String, SoxObject)>,
    pub map_bindings: HashMap<String, SoxObject>,
    pub store_mode: StoreMode,
}


impl Namespace {
    pub(crate) fn new(mode: StoreMode) -> Self {
        let bindings = vec![];
        Self {
            bindings,
            map_bindings: HashMap::new(),
            store_mode: mode,
        }
    }

    pub(crate) fn define(&mut self, key: &EnvKey, value: SoxObject) -> SoxResult<()> {
        match self.store_mode {
            StoreMode::Vec => {
                if let EnvKey::NameIdxPair((name, _, _)) = key {
                    self.bindings.push((name.to_string(), value));
                } else if let EnvKey::Name(name) = key {
                    self.bindings.push((name.to_string(), value));
                }
                Ok(())
            }
            StoreMode::Map => {
                if let EnvKey::Name(name) = key {
                    self.map_bindings.insert(name.to_string(), value);
                }
                Ok(())
            }
        }
    }

    fn assign(&mut self, key: &EnvKey, value: SoxObject) -> SoxResult<()> {
        match self.store_mode {
            StoreMode::Vec => {
                if let EnvKey::NameIdxPair((name, _, binding_idx)) = key {
                    let mut entry = self.bindings.get_mut(*binding_idx);
                    if entry.as_ref().unwrap().0 == *name {
                        entry.as_mut().unwrap().1 = value;
                    }
                } else if let EnvKey::Name(name) = key {
                    for binding in self.bindings.iter_mut() {
                        if binding.0.to_string().eq(name) {
                            binding.1 = value;
                            break;
                        }
                    }
                }
                Ok(())
            }

            StoreMode::Map => {
                if let EnvKey::Name(name) = key {
                    self.map_bindings.insert(name.into(), value);
                }
                Ok(())
            }
        }
    }


    fn get(&mut self, key: &EnvKey) -> SoxResult<SoxObject> {
        match self.store_mode {
            StoreMode::Vec => {
                if let EnvKey::NameIdxPair((name, _, idx)) = key {
                    let value = self.bindings.get(*idx);
                    let ret_val = if let Some(v) = value {
                        Ok(v.1.clone())
                    } else {
                        Err(Exception::Err(RuntimeError {
                            msg: format!("NameError: name '{name}' is not defined"),
                        })
                            .into_ref())
                    };
                    ret_val
                } else if let EnvKey::Name(name) = key {
                    let mut ret_val = None;
                    for (name_, obj) in self.bindings.iter_mut() {
                        if name_.to_string().eq(name) {
                            ret_val = Some(obj);
                            break;
                        }
                    }
                    let ret_val = if let Some(v) = ret_val {
                        Ok(v.clone())
                    } else {
                        Err(Exception::Err(RuntimeError {
                            msg: format!("NameError: name '{name}' is not defined"),
                        })
                            .into_ref())
                    };
                    ret_val
                } else {
                    Err(Exception::Err(RuntimeError {
                        msg: format!("NameError - incorrect namespace key used."),
                    })
                        .into_ref())
                }
            }
            StoreMode::Map => {
                if let EnvKey::Name(name) = key {
                    let ret_val = if let Some(v) = self.map_bindings.get(name) {
                        Ok(v.clone())
                    } else {
                        Err(Exception::Err(RuntimeError {
                            msg: format!("NameError: name '{name}' is not defined"),
                        })
                            .into_ref())
                    };
                    ret_val
                } else {
                    Err(Exception::Err(RuntimeError {
                        msg: format!("NameError - incorrect namespace key used."),
                    })
                        .into_ref())
                }
            }
        }
    }
}


#[derive(Clone, Debug)]
pub struct Env {
    pub mode: StoreMode,
    pub namespaces: Vec<Namespace>,
}


impl Env {
    pub fn new(mode: StoreMode) -> Self {
        Self {
            mode: mode.clone(),
            namespaces: vec![Namespace::new(mode)],
        }
    }
    pub fn define(&mut self, key: &EnvKey, value: SoxObject) {
        let ns = self.namespaces.last_mut().unwrap();
        let _ = ns.define(key, value);
    }

    pub fn get(&mut self, key: &EnvKey) -> SoxResult {
        let mut val = None;
        let mut name_literal = "".to_string();
        match key.clone() {
            EnvKey::Name(name) => {
                name_literal = name.clone();
                for namespace in self.namespaces.iter_mut().rev() {
                    if let Ok(_) = namespace.get(key) {
                        val = Some(namespace.get(key));
                        break;
                    }
                }
            }
            EnvKey::NameIdxPair((name, idx, _)) => {
                name_literal = name.clone();
                let l = self.namespaces.len();
                let ns = self.namespaces.get_mut(l - idx - 1).unwrap();
                    val = Some(ns.get(key));
            }
        }
        
        if val.is_some() {
            return val.unwrap();
        }
        Err(Exception::Err(RuntimeError {
            msg: format!("NameError: name '{name_literal}' is not defined"),
        })
            .into_ref())
    }

   
    pub fn assign(
        &mut self,
        key: &EnvKey,
        value: SoxObject,
    ) -> SoxResult<()> {

        let mut name_literal = "".to_string();
        match key.clone() {
            EnvKey::Name(name) => {
                name_literal = name.to_string();
                for namespace in self.namespaces.iter_mut().rev() {
                    if let Ok(_) = namespace.get(&key) {
                        namespace.assign(&key, value)?;
                        return Ok(());
                    }
                }
            }
            EnvKey::NameIdxPair((name, idx, binding_idx)) => {
                name_literal = name.to_string();
                let l = self.namespaces.len();

                let ns = self.namespaces.get_mut(l - idx - 1);

                if let Some(ns) = ns {
                    info!("assign {:?} to {:?} in {:?}", key, value, ns);

                    ns.assign(&key, value)?;
                    return Ok(());
                }
            }
        }

        Err(Exception::Err(RuntimeError {
            msg: format!("NameError: Name '{name_literal}' is not defined."),
        })
            .into_ref())
    }

    pub fn new_namespace(&mut self) -> SoxResult<()> {
        let namespace = Namespace::new(self.mode.clone());
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
