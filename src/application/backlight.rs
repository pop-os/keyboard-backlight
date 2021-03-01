use cascade::cascade;
use glib::{clone, subclass};
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use crate::{DaemonBoard, DerefCell, KeyboardColorButton};

static MODE_MAP: &[&str] = &[
    "SOLID_COLOR",
    "PER_KEY",
    "CYCLE_ALL",
    "CYCLE_LEFT_RIGHT",
    "CYCLE_UP_DOWN",
    "CYCLE_OUT_IN",
    "CYCLE_OUT_IN_DUAL",
    "RAINBOW_MOVING_CHEVRON",
    "CYCLE_PINWHEEL",
    "CYCLE_SPIRAL",
    "RAINDROPS",
    "SPLASH",
    "MULTISPLASH",
];

#[derive(Default)]
pub struct BacklightInner {
    board: DerefCell<DaemonBoard>,
    color_button_bin: DerefCell<gtk::Frame>,
    brightness_scale: DerefCell<gtk::Scale>,
    mode_combobox: DerefCell<gtk::ComboBoxText>,
    speed_scale: DerefCell<gtk::Scale>,
}

impl ObjectSubclass for BacklightInner {
    const NAME: &'static str = "S76Backlight";

    type ParentType = gtk::Box;
    type Type = Backlight;
    type Interfaces = ();

    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib::object_subclass!();

    fn new() -> Self {
        Self::default()
    }
}

impl ObjectImpl for BacklightInner {
    fn constructed(&self, obj: &Self::Type) {
        let mode_combobox = cascade! {
            gtk::ComboBoxText::new();
            ..append(Some("SOLID_COLOR"), "Solid Color");
            ..append(Some("PER_KEY"), "Per Key");
            ..append(Some("CYCLE_ALL"), "Cosmic Background");
            ..append(Some("CYCLE_LEFT_RIGHT"), "Horizonal Scan");
            ..append(Some("CYCLE_UP_DOWN"), "Vertical Scan");
            ..append(Some("CYCLE_OUT_IN"), "Event Horizon");
            ..append(Some("CYCLE_OUT_IN_DUAL"), "Binary Galaxies");
            ..append(Some("RAINBOW_MOVING_CHEVRON"), "Spacetime");
            ..append(Some("CYCLE_PINWHEEL"), "Pinwheel Galaxy");
            ..append(Some("CYCLE_SPIRAL"), "Spiral Galaxy");
            ..append(Some("RAINDROPS"), "Elements");
            ..append(Some("SPLASH"), "Splashdown");
            ..append(Some("MULTISPLASH"), "Meteor Shower");
        };

        let speed_scale = cascade! {
            gtk::Scale::with_range(gtk::Orientation::Horizontal, 0., 255., 1.);
            ..set_halign(gtk::Align::Fill);
            ..set_size_request(200, 0);
        };

        let brightness_scale = cascade! {
            gtk::Scale::with_range(gtk::Orientation::Horizontal, 0., 100., 1.);
            ..set_halign(gtk::Align::Fill);
            ..set_size_request(200, 0);

        };

        // XXX add support to ColorButton for changing keyboard
        let color_button_bin = cascade! {
            gtk::Frame::new(None);
            ..set_shadow_type(gtk::ShadowType::None);
            ..set_valign(gtk::Align::Center);
        };

        cascade! {
            obj;
            ..set_orientation(gtk::Orientation::Horizontal);
            ..set_spacing(8);
            ..add(&cascade! {
                gtk::Label::new(Some("Mode:"));
                ..set_halign(gtk::Align::Start);
            });
            ..add(&mode_combobox);
            ..add(&cascade! {
                gtk::Label::new(Some("Speed:"));
                ..set_halign(gtk::Align::Start);
            });
            ..add(&speed_scale);
            ..add(&cascade! {
                gtk::Label::new(Some("Brightness:"));
                ..set_halign(gtk::Align::Start);
            });
            ..add(&brightness_scale);
            ..add(&cascade! {
                gtk::Label::new(Some("Color:"));
                ..set_halign(gtk::Align::Start);
            });
            ..add(&color_button_bin);
        };

        self.color_button_bin.set(color_button_bin);
        self.brightness_scale.set(brightness_scale);
        self.mode_combobox.set(mode_combobox);
        self.speed_scale.set(speed_scale);
    }
}

impl WidgetImpl for BacklightInner {}
impl ContainerImpl for BacklightInner {}
impl BoxImpl for BacklightInner {}

glib::wrapper! {
    pub struct Backlight(ObjectSubclass<BacklightInner>)
        @extends gtk::Box, gtk::Container, gtk::Widget, @implements gtk::Orientable;
}

impl Backlight {
    pub fn new(board: DaemonBoard) -> Self {
        let obj: Self = glib::Object::new(&[]).unwrap();

        let color_button = KeyboardColorButton::new(board.clone(), 0xff);
        obj.inner().color_button_bin.add(&color_button);

        let (mode, speed) = match board.mode() {
            Ok(value) => value,
            Err(err) => {
                error!("Error getting keyboard mode: {}", err);
                (0, 128)
            }
        };

        let mode = MODE_MAP.get(mode as usize).cloned();

        obj.inner().mode_combobox.set_active_id(mode);
        obj.inner()
            .mode_combobox
            .connect_changed(clone!(@weak obj => move |_|
                obj.mode_speed_changed();
            ));

        obj.inner().speed_scale.set_value(speed.into());
        obj.inner()
            .speed_scale
            .connect_value_changed(clone!(@weak obj => move |_|
                obj.mode_speed_changed();
            ));

        let max_brightness = match board.max_brightness() {
            Ok(value) => value as f64,
            Err(err) => {
                error!("{}", err);
                100.0
            }
        };
        obj.inner().brightness_scale.set_range(0.0, max_brightness);

        let brightness = match board.brightness(0xff) {
            Ok(value) => value as f64,
            Err(err) => {
                error!("{}", err);
                0.0
            }
        };

        obj.inner().brightness_scale.set_value(brightness);
        obj.inner()
            .brightness_scale
            .connect_value_changed(clone!(@weak obj => move |_|
                obj.brightness_changed();
            ));

        obj.inner().board.set(board.clone());

        obj
    }

    fn inner(&self) -> &BacklightInner {
        BacklightInner::from_instance(self)
    }

    fn board(&self) -> &DaemonBoard {
        &self.inner().board
    }

    fn mode_speed_changed(&self) {
        if let Some(id) = self.inner().mode_combobox.get_active_id() {
            if let Some(mode) = MODE_MAP.iter().position(|i| id == **i) {
                let speed = self.inner().speed_scale.get_value();
                if let Err(err) = self.board().set_mode(mode as u8, speed as u8) {
                    error!("Error setting keyboard mode: {}", err);
                }
            }
        }
    }

    fn brightness_changed(&self) {
        let value = self.inner().brightness_scale.get_value() as i32;
        if let Err(err) = self.board().set_brightness(0xff, value) {
            error!("{}", err);
        }
        debug!("Brightness: {}", value)
    }
}