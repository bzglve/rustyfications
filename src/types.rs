use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use crate::gui::window::Window;

pub type RuntimeData = Rc<RefCell<_RuntimeData>>;

#[derive(Default)]
pub struct _RuntimeData {
    pub windows: BTreeMap<u32, Window>,
}
