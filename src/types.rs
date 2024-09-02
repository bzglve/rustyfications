use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    rc::Rc,
};

use crate::gui::window::Window;

pub type RuntimeData = Rc<RefCell<_RuntimeData>>;

pub struct _RuntimeData {
    pub windows: BTreeMap<u32, Window>,
    pub icon_redefines: HashMap<String, String>, // TODO it has to be in config
}

impl Default for _RuntimeData {
    fn default() -> Self {
        let mut icon_redefines = HashMap::new();
        icon_redefines.insert("inline-reply".to_owned(), "mail-reply".to_owned());
        icon_redefines.insert("dismiss".to_owned(), "window-close".to_owned());

        Self {
            windows: Default::default(),
            icon_redefines,
        }
    }
}
