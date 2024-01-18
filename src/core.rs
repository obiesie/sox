use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::rc::Rc;


pub type SoxObjectRef = Rc<SoxObject>;

#[derive(Debug)]
pub struct SoxObject{
    type_id: TypeId,
    payload: Box<dyn SoxObjectPayload>
}

impl SoxObject {
    pub fn new<T: SoxObjectPayload + 'static>(payload: T) -> Self {
        Self {
            payload: Box::new(payload),
            type_id: TypeId::of::<T>(),
        }
    }

    pub fn payload<I: SoxObjectPayload + 'static>(&self) -> Option<&I> {
        let any = self.payload.as_any();
        any.downcast_ref::<I>()
    }
}

pub trait SoxObjectPayload: Debug {
    fn as_any(&self) -> &dyn Any;
    fn into_sox_object(self) -> SoxObjectRef;
}
