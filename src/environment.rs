use crate::builtins::exceptions::{Exception, RuntimeError};
use crate::core::{SoxObject, SoxObjectPayload, SoxResult};
use slotmap::{DefaultKey, SecondaryMap, SlotMap};
use std::collections::HashMap;
use std::fmt::Display;

pub(crate) type EnvKey = (String, usize, usize);

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
    const NAME_ERR_MSG: &'static str = "NameError: name '{}' is not defined";

    pub(crate) fn new() -> Self {
        let bindings = vec![];
        Self { bindings }
    }

    pub(crate) fn define<T: ToString + Display>(&mut self, key: T, value: SoxObject) -> SoxResult<()> {
        self.bindings.push((key.to_string(), value));
        Ok(())
    }

    pub(crate) fn assign(&mut self, key: &EnvKey, value: SoxObject) -> SoxResult<()> {
        let (name, _, binding_idx) = key;
        let mut binding = self.get_binding_mut(*binding_idx);
        if binding.as_ref().unwrap().0 == *name {
            binding.as_mut().unwrap().1 = value;
        }
        Ok(())
    }

    fn get_binding_mut(&mut self, idx: usize) -> Option<&mut (String, SoxObject)> {
        self.bindings.get_mut(idx)
    }

    pub(crate) fn get(&mut self, key: &EnvKey) -> SoxResult<SoxObject> {
        let (name, _, binding_idx) = key;
        let binding = self.get_binding(*binding_idx);
        if let Some(v) = binding {
            Ok(v.1.clone())
        } else {
            Err(Exception::Err(RuntimeError {
                msg: format!("{} {}", Self::NAME_ERR_MSG, name),
            }).into_ref())
        }
    }

    fn get_binding(&self, idx: usize) -> Option<&(String, SoxObject)> {
        self.bindings.get(idx)
    }
}


pub struct Environment {
    pub envs: SlotMap<DefaultKey, Namespace>,
    pub active: DefaultKey,
    pub global: DefaultKey,
    pub env_link: HashMap<DefaultKey, DefaultKey>,
    pub env_rc: SecondaryMap<DefaultKey, i32>,
}

impl Environment {
    pub fn new() -> Environment {
        let mut envs = SlotMap::new();
        let global_env = Namespace::new();
        let global_env_ref = envs.insert(global_env);
        let env_rc = SecondaryMap::new();
        Self {
            envs,
            active: global_env_ref,
            global: global_env_ref,
            env_link: Default::default(),
            env_rc,
        }
    }

    pub fn define_at<T: ToString + Display>(&mut self, key: T, value: SoxObject, ns_ref: DefaultKey) {
        let ns = self.envs.get_mut(ns_ref).unwrap();
        let _ = ns.define(key, value);
    }

    fn create_environment(&mut self, enclosing_env_ref: DefaultKey) -> DefaultKey {
        let new_env = Namespace::new();
        let new_env_ref = self.envs.insert(new_env);
        self.env_link.insert(new_env_ref, enclosing_env_ref);

        self.env_rc.entry(enclosing_env_ref)
            .unwrap()
            .and_modify(|v| *v += 1)
            .or_insert(1);

        new_env_ref
    }

    pub fn new_local_env_at(&mut self, enclosing_env_ref: DefaultKey) -> DefaultKey {
        self.create_environment(enclosing_env_ref)
    }

    pub fn new_local_env(&mut self) -> DefaultKey {
        self.active = self.create_environment(self.active);
        self.active
    }

    pub fn new_local_env_unused(&mut self) -> DefaultKey {
        self.create_environment(self.active)
    }

    pub fn define<T: ToString + Display>(&mut self, key: T, value: SoxObject) {
        let ns = self.envs.get_mut(self.active).unwrap();
        let _ = ns.define(key, value);
    }

    pub fn get_from_global_scope(&self, key: String) -> SoxResult {
        let key_string = key.to_string();
        let global_namespace = self.envs.get(self.global).unwrap();
        match global_namespace.bindings.iter().find(|v| v.0 == key_string) {
            Some(v) => Ok(v.1.clone()),
            None => Err(Exception::Err(RuntimeError {
                msg: format!("NameError: name '{key_string}' is not defined."),
            }).into_ref())
        }
    }

    pub fn get(&mut self, key: EnvKey) -> SoxResult {
        let (ref name, dist_to_ns, _) = key;
        let mut namespace = self.envs.get_mut(self.active).unwrap();
        let mut namespace_ref = self.active;
        let mut dist = 0;

        while dist < dist_to_ns {
            match self.env_link.get(&namespace_ref) {
                Some(&parent_ns) => {
                    namespace_ref = parent_ns;
                    namespace = self.envs.get_mut(parent_ns).unwrap();
                }
                None => return Err(Exception::Err(RuntimeError {
                    msg: format!("NameError: name '{name}' is not defined"),
                }).into_ref())
            }
            dist += 1;
        }
        namespace.get(&key)
    }

    pub fn find_and_get<T: ToString + Display>(&mut self, key: T) -> SoxResult {
        let key_string = key.to_string();
        let mut current_ns_key = Some(self.active);
        while let Some(namespace_key) = current_ns_key {
            let namespace = self.envs.get_mut(namespace_key).unwrap();
            if let Some(value) = namespace.bindings.iter_mut().find_map(|(k, v)| {
                if *k == key_string { Some(v.clone()) } else { None }
            }) {
                return Ok(value);
            }
            current_ns_key = self.env_link.get(&namespace_key).copied();
        }
        Err(Exception::Err(RuntimeError {
            msg: format!("NameError: name '{key_string}' is not defined"),
        }).into_ref())
    }

    pub fn find_and_assign<T: ToString + Display>(
        &mut self,
        key: T,
        value: SoxObject,
    ) -> SoxResult<()> {
        let key_string = key.to_string();
        let mut ns_key = Some(self.active);
        while let Some(nsk) = ns_key {
            let ns = self.envs.get_mut(nsk).unwrap();
            if let Some(v) = ns.bindings.iter_mut().find(|v| v.0 == key_string) {
                v.1 = value;
                return Ok(());
            }
            ns_key = self.env_link.get(&nsk).copied();
        }
        Err(Exception::Err(RuntimeError {
            msg: format!("NameError: name '{key_string}' is not defined."),
        }).into_ref())
    }

    pub fn assign(&mut self, key: &EnvKey, value: SoxObject) -> SoxResult<()> {
        let (_, mut dist_to_ns, _) = key;
        let mut ns_key = Some(self.active);
        while dist_to_ns > 0 {
            ns_key = self.env_link.get(ns_key.as_ref().unwrap()).copied();
            dist_to_ns -= 1;
        }
        let ns = self.envs.get_mut(ns_key.unwrap()).unwrap();
        ns.assign(&key, value)?;
        Ok(())
    }

    pub fn pop(&mut self) -> SoxResult<()> {
        let (active, parent) = (self.active, self.env_link.get(&self.active).unwrap());
        self.active = *parent;
        if self.env_rc.entry(active).unwrap().or_default() == &0 {
            self.envs.remove(active);
            self.env_rc.entry(*parent).unwrap().and_modify(|v| *v -= 1);
            self.env_link.remove(&active);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::int::SoxInt;

    #[test]
    fn test_environment_assign() {
        let mut env = Environment::new();
        let key = "key".to_string();
        let value1 = SoxInt::new(10).into_ref();
        let value2 = SoxInt::new(20).into_ref();

        // Define the key in the global environment
        env.define(key.clone(), value1.clone());

        // Assign a new value to the key
        let env_key = (key.clone(), 0, 0);
        let result = env.assign(&env_key, value2.clone());
        assert!(result.is_ok());

        // Retrieve the value to check if it has been updated
        let retrieved_value = env.get(env_key);
        assert!(retrieved_value.is_ok());
        let i = retrieved_value.unwrap().as_int().unwrap().value;
        assert_eq!(i, 20);
    }
    #[test]
    fn test_environment_define_and_get() {
        let mut env = Environment::new();
        let key = "key".to_string();
        let value = SoxInt::new(10).into_ref(); // Assuming SoxObject has a `new` method
        let value2 = SoxInt::new(20).into_ref();
        let value3 = SoxInt::new(30).into_ref();

        env.define(key.clone(), value.clone());
        env.define("key2".to_string(), value2.clone());

        let t = env.envs.get(env.active).unwrap();
        let new_env1 = env.new_local_env();
        env.active = new_env1;
        env.define("key3".to_string(), value3);

        let new_env2 = env.new_local_env();
        env.active = new_env2;

        let env_key = (key.clone(), 2, 0); // replace "x" if needed
        let result = env.get(env_key);
        assert!(result.is_ok());
        let retrieved_value = result.unwrap();
        let i = retrieved_value.as_int().unwrap().value;
        assert_eq!(i, 10);

        let env_key = ("key2".to_string(), 2, 1); // replace "x" if needed
        let result = env.get(env_key);
        assert!(result.is_ok());
        let retrieved_value = result.unwrap();
        let i = retrieved_value.as_int().unwrap().value;
        assert_eq!(i, 20);

        let env_key = ("key3".to_string(), 1, 0); // replace "x" if needed
        let result = env.get(env_key);
        assert!(result.is_ok());
        let retrieved_value = result.unwrap();
        let i = retrieved_value.as_int().unwrap().value;
        assert_eq!(i, 30);
    }
}