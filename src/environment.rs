use std::collections::HashMap;
use std::fmt::Display;

use log::debug;

use crate::core::SoxObject;
use crate::exceptions::RuntimeException;

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

    pub fn get<T: AsRef<str>>(&mut self, name: T) -> Result<SoxObject, RuntimeException> {
        let ret_val = if let Some(v) = self.bindings.get(name.as_ref()) {
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
    pub fn define<T: Into<String>>(&mut self, name: T, value: SoxObject) {
        let _ = self.namespaces.last_mut().unwrap().assign(name, value);
    }

    pub fn get<T: Into<String> + Display>(&mut self, name: T) -> Result<SoxObject, RuntimeException> {
        let name_literal = name.into();
        for namespace in self.namespaces.iter_mut().rev() {
            if let Ok(value) = namespace.get(name_literal.as_str()) {
                return Ok(value.clone());
            }
        }

        return Err(RuntimeException {
            msg: format!("Undefined variable {}.", name_literal),
        });
    }

    pub fn assign<T: Into<String> + Display>(&mut self, name: T, value: SoxObject) -> Result<(), RuntimeException> {
        let name_literal = name.into();
        for namespace in self.namespaces.iter_mut().rev() {
            if let Ok(_) = namespace.get(name_literal.as_str()) {
                namespace.assign(name_literal, value)?;
                return Ok(());
            }
        }

        return Err(RuntimeException {
            msg: format!("Variable {} not defined in curr env.", name_literal),
        });
    }

    pub fn new_namespace(&mut self) -> Result<(), RuntimeException> {
        let namespace = Namespace::default();
        let _ = self.push(namespace)?;

        Ok(())
    }

    pub fn push(&mut self, namespace: Namespace) -> Result<(), RuntimeException> {
        self.namespaces.push(namespace);
        Ok(())
    }

    pub fn pop(&mut self) -> Result<(), RuntimeException> {
        self.namespaces.pop();
        Ok(())
    }

    pub fn size(&self) -> usize {
        self.namespaces.len()
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::core::SoxObject;
    use crate::environment::Env;
    use crate::int::SoxInt;
    use crate::payload;
    use crate::string::SoxString;

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
    fn test_assign_without_def(){
        let mut env = Env::default();
        let obj = SoxInt::new(10).into_sox_obj();
        let r = env.assign("undefined", obj);

        assert!(r.is_err());
    }
    #[test]
    fn test_assignment_to_namespace() {
        let mut env = Env::default();
        let obj = SoxInt::new(10).into_sox_obj();

        env.define("test_obj", obj);

        let obj_ref = env.get("test_obj");
        assert!(obj_ref.is_ok());

        let obj = obj_ref.unwrap();
        let sox_type = payload!(obj, SoxObject::Int).unwrap();
        assert_eq!(sox_type.value, 10);

        assert_eq!(Rc::strong_count(&sox_type), 2);
    }

    #[test]
    fn test_multi_namespace_assignment() {
        let mut env = Env::default();
        let obj = SoxInt::new(10).into_sox_obj();
        let another_obj = SoxString::new("hello world").into_sox_obj();

        env.define("test_obj", obj);

        let _ = env.new_namespace();
        env.define("test_obj", another_obj);

        let obj_ref = env.get("test_obj");
        assert!(obj_ref.is_ok());

        let obj = obj_ref.unwrap();
        let sox_type = payload!(obj, SoxObject::String).unwrap();
        assert_eq!(sox_type.value, String::from("hello world"));

        assert_eq!(Rc::strong_count(&sox_type), 2);

        let _ = env.pop();

        let obj_ref = env.get("test_obj");
        assert!(obj_ref.is_ok());

        let obj = obj_ref.unwrap();
        let sox_type = payload!(obj, SoxObject::Int).unwrap();
        assert_eq!(sox_type.value, 10);

        assert_eq!(Rc::strong_count(&sox_type), 2);
    }
}

