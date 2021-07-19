use cascade::cascade;
use futures::{prelude::*, stream::FuturesUnordered};
use glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use std::{cell::RefCell, collections::HashMap};

use crate::Keyboard;
use backend::DerefCell;

mod picker_group;
mod picker_group_box;
mod picker_json;
mod picker_key;

use picker_group_box::PickerGroupBox;
use picker_json::picker_json;
use picker_key::PickerKey;

pub static SCANCODE_LABELS: Lazy<HashMap<String, String>> = Lazy::new(|| {
    let mut labels = HashMap::new();
    for group in picker_json() {
        for key in group.keys {
            labels.insert(key.keysym, key.label);
        }
    }
    labels
});

#[derive(Default)]
pub struct PickerInner {
    group_box: DerefCell<PickerGroupBox>,
    keyboard: RefCell<Option<Keyboard>>,
    mod_tap_box: DerefCell<gtk::Box>,
    mod_tap_check: DerefCell<gtk::CheckButton>,
    mod_tap_mods: DerefCell<gtk::ComboBoxText>,
}

#[glib::object_subclass]
impl ObjectSubclass for PickerInner {
    const NAME: &'static str = "S76KeyboardPicker";
    type ParentType = gtk::Box;
    type Type = Picker;
}

impl ObjectImpl for PickerInner {
    fn constructed(&self, picker: &Picker) {
        self.parent_constructed(picker);

        let group_box = cascade! {
            PickerGroupBox::new();
            ..connect_key_pressed(clone!(@weak picker => move |name| {
                picker.key_pressed(name)
            }));
        };

        // TODO: set initial values, bind change

        let mod_tap_check = cascade! {
            gtk::CheckButton::with_label("Mod-Tap");
            ..connect_toggled(clone!(@weak picker => move |_| {
                picker.mod_tap_updated();
            }));
        };

        let mod_tap_mods = cascade! {
            gtk::ComboBoxText::new();
            ..append(Some("LCTL"), "Left Ctrl");
            ..append(Some("LSFT"), "Left Shift");
            ..append(Some("LALT"), "Left Alt");
            ..append(Some("LGUI"), "Left Super");
            ..append(Some("RCTL"), "Right Ctrl");
            ..append(Some("RSFT"), "Right Shift");
            ..append(Some("RALT"), "Right Alt");
            ..append(Some("RGUI"), "Right Super");
            ..connect_property_active_id_notify(clone!(@weak picker => move |_| {
                picker.mod_tap_updated();
            }));
        };

        let mod_tap_box = cascade! {
            gtk::Box::new(gtk::Orientation::Horizontal, 8);
            ..add(&mod_tap_check);
            ..add(&mod_tap_mods);
        };

        cascade! {
            picker;
            ..set_spacing(18);
            ..set_orientation(gtk::Orientation::Vertical);
            ..add(&group_box);
            ..add(&mod_tap_box);
            ..show_all();
        };

        self.group_box.set(group_box);
        self.mod_tap_box.set(mod_tap_box);
        self.mod_tap_check.set(mod_tap_check);
        self.mod_tap_mods.set(mod_tap_mods);
    }
}

impl BoxImpl for PickerInner {}

impl WidgetImpl for PickerInner {}

impl ContainerImpl for PickerInner {}

glib::wrapper! {
    pub struct Picker(ObjectSubclass<PickerInner>)
        @extends gtk::Box, gtk::Container, gtk::Widget, @implements gtk::Orientable;
}

impl Picker {
    pub fn new() -> Self {
        glib::Object::new(&[]).unwrap()
    }

    fn inner(&self) -> &PickerInner {
        PickerInner::from_instance(self)
    }

    pub(crate) fn set_keyboard(&self, keyboard: Option<Keyboard>) {
        if let Some(old_kb) = &*self.inner().keyboard.borrow() {
            old_kb.set_picker(None);
        }

        if let Some(kb) = &keyboard {
            self.inner().group_box.set_keyboard(&kb);
            kb.set_picker(Some(&self));
        }

        *self.inner().keyboard.borrow_mut() = keyboard;
    }

    pub(crate) fn set_selected(&self, scancode_names: Vec<String>) {
        // TODO selected needs to support mod tap
        self.inner().group_box.set_selected(scancode_names);
    }

    fn key_pressed(&self, name: String) {
        let kb = match self.inner().keyboard.borrow().clone() {
            Some(kb) => kb,
            None => {
                return;
            }
        };
        let layer = kb.layer();

        info!("Clicked {} layer {:?}", name, layer);
        if let Some(layer) = layer {
            let futures = FuturesUnordered::new();
            for i in kb.selected().iter() {
                let i = *i;
                futures.push(clone!(@strong kb, @strong name => async move {
                    kb.keymap_set(i, layer, &name).await;
                }));
            }
            glib::MainContext::default().spawn_local(async { futures.collect::<()>().await });
        }
    }

    fn mod_tap_updated(&self) {
        fn mt(mod_: u16, kc: u16) -> u16 {
            0x6000 | ((mod_ & 0x1f) << 8) | (kc & 0xff)
        }

        //MT(mod, kc) (QK_MOD_TAP | (((mod)&0x1F) << 8) | ((kc)&0xFF))
        let active = self.inner().mod_tap_check.get_active();
        let kc: u16 = match self.inner().mod_tap_mods.get_active_id().as_deref() {
            Some("LCTL") => 0x01,
            Some("LSFT") => 0x02,
            Some("LALT") => 0x04,
            Some("LGUI") => 0x08,
            Some("RCTL") => 0x11,
            Some("RSFT") => 0x12,
            Some("RALT") => 0x14,
            Some("RGUI") => 0x18,
            Some(_) => unreachable!(),
            None => {
                return;
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use backend::{layouts, Layout};
    use std::collections::HashSet;

    #[test]
    fn picker_has_keys() {
        let mut missing = HashSet::new();
        for i in layouts() {
            let layout = Layout::from_board(i).unwrap();
            for j in layout.default.map.values().flatten() {
                if SCANCODE_LABELS.keys().find(|x| x == &j).is_none() {
                    missing.insert(j.to_owned());
                }
            }
        }
        assert_eq!(missing, HashSet::new());
    }
}
