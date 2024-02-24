use std::collections::HashMap;
use std::fmt::Display;

use log::debug;

use crate::core::SoxObject;
use crate::exceptions::RuntimeError;

#[derive(Clone, Debug)]
struct Namespace {
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
    pub fn define<T: Into<String>>(
        &mut self,
        name: T,
        value: SoxObject,
    ) -> Result<(), RuntimeError> {
        self.bindings.insert(name.into(), value);
        Ok(())
    }

    pub fn assign<T: Into<String>>(
        &mut self,
        name: T,
        value: SoxObject,
    ) -> Result<(), RuntimeError> {
        self.bindings.insert(name.into(), value);
        Ok(())
    }

    pub fn get<T: AsRef<str> + Display>(&mut self, name: T) -> Result<SoxObject, RuntimeError> {
        let ret_val = if let Some(v) = self.bindings.get(name.as_ref()) {
            Ok(v.clone())
        } else {
            Err(RuntimeError { msg: format!("NameError: name '{name}' is not defined") })
        };
        ret_val
    }
}

#[derive(Clone, Debug)]
pub struct Env {
    namespaces: Vec<Namespace>,
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

    pub fn get<T: Into<String> + Display>(&mut self, name: T) -> Result<SoxObject, RuntimeError> {
        let name_literal = name.into();
        for namespace in self.namespaces.iter_mut().rev() {
            if let Ok(value) = namespace.get(name_literal.as_str()) {
                return Ok(value.clone());
            }
        }

        return Err(RuntimeError {
            msg:format!("NameError: name '{name_literal}' is not defined"),
        });
    }

    pub fn assign<T: Into<String> + Display>(
        &mut self,
        name: T,
        value: SoxObject,
    ) -> Result<(), RuntimeError> {
        let name_literal = name.into();
        for namespace in self.namespaces.iter_mut().rev() {
            if let Ok(_) = namespace.get(name_literal.as_str()) {
                namespace.assign(name_literal, value)?;
                return Ok(());
            }
        }

        return Err(RuntimeError {
            msg: format!("NameError: Name '{name_literal}' is not defined."),
        });
    }

    pub fn new_namespace(&mut self) -> Result<(), RuntimeError> {
        let namespace = Namespace::default();
        let _ = self.namespaces.push(namespace);

        Ok(())
    }

    pub fn pop(&mut self) -> Result<(), RuntimeError> {
        self.namespaces.pop();
        Ok(())
    }

    pub fn size(&self) -> usize {
        self.namespaces.len()
    }
}

#[cfg(test)]
mod tests {
    use crate::core::SoxObject;
    use crate::environment::Env;
    use crate::payload;

    #[test]
    fn test_empty_env() {
        let env = Env::default();
        assert_eq!(env.size(), 1);
    }

    #[test]
    fn test_get_missing_identifier() {
        let mut env = Env::default();
        let obj = env.get("missing");

        assert!(obj.is_err());
    }

    #[test]
    fn test_assign_without_def() {
        let mut env = Env::default();
        let obj = SoxObject::Int(1);
        let r = env.assign("undefined", obj);
    
        assert!(r.is_err());
    }
    
    #[test]
    fn test_assignment_to_namespace() {
        let mut env = Env::default();
        let obj = SoxObject::Int(10);
    
        env.define("test_obj", obj);
    
        let obj_ref = env.get("test_obj");
        assert!(obj_ref.is_ok());
    
        let obj = obj_ref.unwrap();
        let sox_obj = payload!(obj, SoxObject::Int).unwrap();
        assert_eq!(sox_obj, 10);
    
    }
    
    #[test]
    fn test_multi_namespace_assignment() {
        let mut env = Env::default();
        let obj = SoxObject::Int(10);
        let another_obj = SoxObject::String("hello world".into());
    
        env.define("test_obj", obj);
    
        let _ = env.new_namespace();
        env.define("test_obj", another_obj);
    
        let obj_ref = env.get("test_obj");
        assert!(obj_ref.is_ok());
    
        let obj = obj_ref.unwrap();
        let sox_obj = payload!(obj, SoxObject::String).unwrap();
        assert_eq!(sox_obj, String::from("hello world"));
    
        let _ = env.pop();
    
        let obj_ref = env.get("test_obj");
        assert!(obj_ref.is_ok());
    
        let obj = obj_ref.unwrap();
        let sox_obj = payload!(obj, SoxObject::Int).unwrap();
        assert_eq!(sox_obj, 10);
    
    }
}
