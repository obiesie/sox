use crate::builtins::exceptions::{Exception, RuntimeError};
use crate::core::{SoxObject, SoxObjectPayload, SoxResult};
use log::{debug, info};
use slotmap::secondary::Entry;
use slotmap::{DefaultKey, SecondaryMap, SlotMap};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Display;
use std::rc::Rc;

pub(crate) type EnvKey = (String, usize, usize);
pub type EnvRef = Rc<DefaultKey>;

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
            debug!("Bindings are {:#?}", self.bindings);
            Err(Exception::Err(RuntimeError {
                msg: format!("NameError: name '{}' is not defined", name),
            })
            .into_ref())
        }
    }

    fn get_binding(&self, idx: usize) -> Option<&(String, SoxObject)> {
        self.bindings.get(idx)
    }
}

impl Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Namespace")
            .field("bindings", &self.bindings)
            .finish()
    }
}

pub struct Environment {
    pub envs: SlotMap<DefaultKey, Namespace>,
    pub active: EnvRef,
    pub global: EnvRef,
    pub env_link: HashMap<EnvRef, EnvRef>,
}

impl Environment {
    pub fn stack_new_env(&mut self, ns: Namespace) -> EnvRef {
        let env_ref = self.envs.insert(ns);
        let env_ref = Rc::new(env_ref);
        env_ref
    }
    pub fn new() -> Environment {
        let mut envs = SlotMap::new();
        let global_env = Namespace::new();
        let global_env_ref = envs.insert(global_env);
        let global_env_ref = Rc::new(global_env_ref);
        //let env_rc = SecondaryMap::new();
        Self {
            envs,
            active: global_env_ref.clone(),
            global: global_env_ref,
            env_link: Default::default(),
            //env_rc,
        }
    }

    pub fn define_at<T: ToString + Display>(&mut self, key: T, value: SoxObject, ns_ref: EnvRef) {
        let ns = self.envs.get_mut(*ns_ref).unwrap();
        let _ = ns.define(key, value);
    }

    fn create_environment(&mut self, enclosing_env_ref: EnvRef) -> EnvRef {
        let new_env = Namespace::new();
        let new_env_ref = self.stack_new_env(new_env);
        self.env_link.insert(new_env_ref.clone(), enclosing_env_ref);

        new_env_ref
    }

    pub fn new_local_env_at(&mut self, enclosing_env_ref: EnvRef) -> EnvRef {
        self.create_environment(enclosing_env_ref)
    }

    pub fn new_local_env(&mut self) -> EnvRef {
        self.active = self.create_environment(self.active.clone());
        self.active.clone()
    }

    pub fn new_local_env_unused(&mut self) -> EnvRef {
        self.create_environment(self.active.clone())
    }

    pub fn define<T: ToString + Display>(&mut self, key: T, value: SoxObject) {
        let ns = self.envs.get_mut(*self.active).unwrap();
        let _ = ns.define(key, value);
    }

    pub fn get_from_global_scope(&self, key: String) -> SoxResult {
        let key_string = key.to_string();
        let global_namespace = self.envs.get(*self.global).unwrap();
        match global_namespace.bindings.iter().find(|v| v.0 == key_string) {
            Some(v) => Ok(v.1.clone()),
            None => Err(Exception::Err(RuntimeError {
                msg: format!("NameError: name '{key_string}' is not defined."),
            })
            .into_ref()),
        }
    }

    pub fn get(&mut self, key: EnvKey) -> SoxResult {
        let (ref name, dist_to_ns, _) = key;
        let mut namespace = self.envs.get_mut(*self.active).unwrap();
        let mut namespace_ref = self.active.clone();
        let mut dist = 0;

        while dist < dist_to_ns {
            match self.env_link.get(&namespace_ref) {
                Some(&ref parent_ns) => {
                    namespace_ref = parent_ns.clone();
                    // info!("Fetching parent namespace {:?}", namespace_ref);
                    namespace = self.envs.get_mut(**parent_ns).unwrap();
                }
                None => {
                    return Err(Exception::Err(RuntimeError {
                        msg: format!("NameError: name '{:?}' is not defined", name),
                    })
                    .into_ref())
                }
            }
            dist += 1;
        }
        // info!("The env link is {:?} and dist is {:?}", self.env_link, dist_to_ns);

        let val = namespace.get(&key);
        val
    }

    pub fn find_and_get<T: ToString + Display>(&mut self, key: T) -> SoxResult {
        let key_string = key.to_string();
        let mut current_ns_key = Some(self.active.clone());
        while let Some(namespace_key) = current_ns_key {
            let namespace = self.envs.get_mut(*namespace_key).unwrap();
            if let Some(value) = namespace.bindings.iter_mut().find_map(|(k, v)| {
                if *k == key_string {
                    Some(v.clone())
                } else {
                    None
                }
            }) {
                return Ok(value);
            }
            current_ns_key = self.env_link.get(&namespace_key).cloned();
        }
        Err(Exception::Err(RuntimeError {
            msg: format!("NameError: name '{key_string}' is not defined"),
        })
        .into_ref())
    }

    pub fn find_and_assign<T: ToString + Display>(
        &mut self,
        key: T,
        value: SoxObject,
    ) -> SoxResult<()> {
        let key_string = key.to_string();
        let mut ns_key = Some(self.active.clone());
        while let Some(nsk) = ns_key {
            let ns = self.envs.get_mut(*nsk).unwrap();
            if let Some(v) = ns.bindings.iter_mut().find(|v| v.0 == key_string) {
                v.1 = value;
                return Ok(());
            }
            ns_key = self.env_link.get(&nsk).cloned();
        }
        Err(Exception::Err(RuntimeError {
            msg: format!("NameError: name '{key_string}' is not defined."),
        })
        .into_ref())
    }

    pub fn assign_in_global<T: ToString + Display>(
        &mut self,
        key: T,
        value: SoxObject,
    ) -> SoxResult<()> {
        let key_string = key.to_string();
        let global_ns = self.envs.get_mut(*self.global).unwrap();
        if let Some(v) = global_ns.bindings.iter_mut().find(|v| v.0 == key_string) {
            v.1 = value;
            return Ok(());
        }

        Err(Exception::Err(RuntimeError {
            msg: format!("NameError: name '{key_string}' is not defined."),
        })
        .into_ref())
    }

    pub fn assign(&mut self, key: &EnvKey, value: SoxObject) -> SoxResult<()> {
        let (_, mut dist_to_ns, _) = key;
        let mut ns_key = Some(self.active.clone());
        while dist_to_ns > 0 {
            ns_key = self.env_link.get(ns_key.as_ref().unwrap()).cloned();
            dist_to_ns -= 1;
        }
        let ns = self.envs.get_mut(*ns_key.unwrap()).unwrap();
        ns.assign(&key, value)?;
        Ok(())
    }

    pub fn pop(&mut self) -> SoxResult<()> {
        let (active, parent) = (
            self.active.clone(),
            self.env_link.get(&self.active).unwrap(),
        );
        self.active = parent.clone();
        // check that strong reference count is just from the assignment above and self.envs in which case we can drop the env
        if Rc::strong_count(&active) == 2 {
            self.envs.remove(*active);
            self.env_link.remove(&active);
            // info!("Removed {active:?} from environment - {:?}", self.env_link);
        }

        Ok(())
    }
}
