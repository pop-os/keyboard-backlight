use crate::fl;
use backend::{Board, DerefCell, Rgb, SelmaKind};
use cascade::cascade;
use futures::{
    future::{abortable, AbortHandle},
    prelude::*,
    stream::FuturesUnordered,
};
use glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use once_cell::sync::OnceCell;
use std::{cell::RefCell, collections::HashMap, sync::RwLock};

struct TestResults {
    bench: RwLock<HashMap<&'static str, Result<f64, String>>>,
}

impl TestResults {
    fn global() -> &'static Self {
        static TEST_RESULTS: OnceCell<TestResults> = OnceCell::new();
        TEST_RESULTS.get_or_init(Self::new)
    }

    fn new() -> Self {
        let test_results = Self {
            bench: RwLock::new(HashMap::new()),
        };
        test_results.reset();
        test_results
    }

    fn reset(&self) {
        let mut bench = self.bench.write().unwrap();
        bench.clear();
        for port_desc in &[
            "USB 2.0: USB-A Left",
            "USB 2.0: USB-A Right",
            "USB 2.0: USB-C Left",
            "USB 2.0: USB-C Right",
            "USB 3.2 Gen 2: USB-A Left",
            "USB 3.2 Gen 2: USB-A Right",
            "USB 3.2 Gen 2: USB-C Left",
            "USB 3.2 Gen 2: USB-C Right",
        ] {
            bench.insert(*port_desc, Err("no benchmarks performed".to_string()));
        }
    }
}

#[derive(Clone, Default, glib::GBoxed)]
#[gboxed(type_name = "S76TestingColor")]
pub struct TestingColors(pub HashMap<(usize, usize), Rgb>);

#[derive(Default)]
pub struct TestingInner {
    board: DerefCell<Board>,
    reset_button: DerefCell<gtk::Button>,
    bench_button: DerefCell<gtk::ToggleButton>,
    bench_labels: DerefCell<HashMap<&'static str, gtk::Label>>,
    start_buttons: DerefCell<[gtk::Button; 3]>,
    stop_buttons: DerefCell<[gtk::Button; 3]>,
    test_labels: DerefCell<[gtk::Label; 3]>,
    test_abort_handles: RefCell<[Option<AbortHandle>; 3]>,
    colors: RefCell<TestingColors>,
}

#[glib::object_subclass]
impl ObjectSubclass for TestingInner {
    const NAME: &'static str = "S76Testing";
    type ParentType = gtk::Box;
    type Type = Testing;
}

impl ObjectImpl for TestingInner {
    fn constructed(&self, obj: &Self::Type) {
        fn row(widget: &impl IsA<gtk::Widget>) -> gtk::ListBoxRow {
            cascade! {
                gtk::ListBoxRow::new();
                ..set_selectable(false);
                ..set_activatable(false);
                ..set_property_margin(8);
                ..add(widget);
            }
        }

        fn label_row(label: &str, widget: &impl IsA<gtk::Widget>) -> gtk::ListBoxRow {
            row(&cascade! {
                gtk::Box::new(gtk::Orientation::Horizontal, 8);
                ..add(&cascade! {
                    gtk::Label::new(Some(label));
                    ..set_halign(gtk::Align::Start);
                });
                ..pack_end(widget, false, false, 0);
            })
        }

        fn color_box(r: f64, g: f64, b: f64) -> gtk::DrawingArea {
            cascade! {
                gtk::DrawingArea::new();
                ..set_size_request(18, 18);
                ..connect_draw(move |_w, cr| {
                    cr.set_source_rgb(r, g, b);
                    cr.paint();
                    Inhibit(false)
                });
            }
        }

        {
            let reset_button = gtk::Button::with_label("Reset testing");

            obj.add(&cascade! {
                gtk::ListBox::new();
                ..set_valign(gtk::Align::Start);
                ..get_style_context().add_class("frame");
                ..add(&row(&reset_button));
            });

            self.reset_button.set(reset_button);
        }

        {
            let list = gtk::ListBox::new();

            let mut bench_labels = HashMap::new();
            for (port_desc, _port_result) in TestResults::global().bench.read().unwrap().iter() {
                let bench_label = gtk::Label::new(None);
                list.add(&label_row(port_desc, &bench_label));
                bench_labels.insert(*port_desc, bench_label);
            }

            let bench_button = gtk::ToggleButton::with_label("Run USB test");

            obj.add(&cascade! {
                gtk::Box::new(gtk::Orientation::Vertical, 12);
                ..add(&gtk::Label::new(Some("USB Port Test")));
                ..add(&cascade! {
                    list;
                    ..set_valign(gtk::Align::Start);
                    ..get_style_context().add_class("frame");
                    ..add(&row(&bench_button));
                    ..set_header_func(Some(Box::new(|row, before| {
                        if before.is_none() {
                            row.set_header::<gtk::Widget>(None)
                        } else if row.get_header().is_none() {
                            row.set_header(Some(&cascade! {
                                gtk::Separator::new(gtk::Orientation::Horizontal);
                                ..show();
                            }));
                        }
                    })));
                });
            });

            self.bench_button.set(bench_button);
            self.bench_labels.set(bench_labels);
        }

        let start_buttons = [
            gtk::Button::with_label(&fl!("test-start")),
            gtk::Button::with_label(&fl!("test-start")),
            gtk::Button::with_label(&fl!("test-start")),
        ];
        let stop_buttons = [
            gtk::Button::with_label(&fl!("test-stop")),
            gtk::Button::with_label(&fl!("test-stop")),
            gtk::Button::with_label(&fl!("test-stop")),
        ];
        let test_labels = [
            gtk::Label::new(None),
            gtk::Label::new(None),
            gtk::Label::new(None),
        ];

        obj.add(&cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 12);
            ..add(&gtk::Label::new(Some("Selma Test 1")));
            ..add(&cascade! {
                gtk::ListBox::new();
                ..set_valign(gtk::Align::Start);
                ..get_style_context().add_class("frame");
                ..add(&row(&cascade! {
                    gtk::Box::new(gtk::Orientation::Horizontal, 8);
                    ..set_halign(gtk::Align::Center);
                    ..add(&start_buttons[0]);
                    ..add(&stop_buttons[0]);
                }));
                ..add(&row(&test_labels[0]));
                ..add(&label_row(&fl!("test-check-pins"), &color_box(1., 0., 0.)));
                ..add(&label_row(&fl!("test-check-key"), &color_box(0., 1., 0.)));
                ..set_header_func(Some(Box::new(|row, before| {
                    if before.is_none() {
                        row.set_header::<gtk::Widget>(None)
                    } else if row.get_header().is_none() {
                        row.set_header(Some(&cascade! {
                            gtk::Separator::new(gtk::Orientation::Horizontal);
                            ..show();
                        }));
                    }
                })));
            });
        });

        obj.add(&cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 12);
            ..add(&gtk::Label::new(Some("Selma Test 2")));
            ..add(&cascade! {
                gtk::ListBox::new();
                ..set_valign(gtk::Align::Start);
                ..get_style_context().add_class("frame");
                ..add(&row(&cascade! {
                    gtk::Box::new(gtk::Orientation::Horizontal, 8);
                    ..add(&start_buttons[1]);
                    ..add(&stop_buttons[1]);
                }));
                ..add(&row(&test_labels[1]));
                ..add(&label_row(&fl!("test-replace-switch"), &color_box(0., 0., 1.)));
                ..set_header_func(Some(Box::new(|row, before| {
                    if before.is_none() {
                        row.set_header::<gtk::Widget>(None)
                    } else if row.get_header().is_none() {
                        row.set_header(Some(&cascade! {
                            gtk::Separator::new(gtk::Orientation::Horizontal);
                            ..show();
                        }));
                    }
                })));
            });
        });

        obj.add(&cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 12);
            ..add(&gtk::Label::new(Some("Selma Test 3")));
            ..add(&cascade! {
                gtk::ListBox::new();
                ..set_valign(gtk::Align::Start);
                ..get_style_context().add_class("frame");
                ..add(&row(&cascade! {
                    gtk::Box::new(gtk::Orientation::Horizontal, 8);
                    ..add(&start_buttons[2]);
                    ..add(&stop_buttons[2]);
                }));
                ..add(&row(&test_labels[2]));
                ..add(&label_row(&fl!("test-check-pins"), &color_box(1., 0., 0.)));
                ..add(&label_row(&fl!("test-check-key"), &color_box(0., 1., 0.)));
                ..set_header_func(Some(Box::new(|row, before| {
                    if before.is_none() {
                        row.set_header::<gtk::Widget>(None)
                    } else if row.get_header().is_none() {
                        row.set_header(Some(&cascade! {
                            gtk::Separator::new(gtk::Orientation::Horizontal);
                            ..show();
                        }));
                    }
                })));
            });
        });

        self.start_buttons.set(start_buttons);
        self.stop_buttons.set(stop_buttons);
        self.test_labels.set(test_labels);

        cascade! {
            obj;
            ..set_orientation(gtk::Orientation::Vertical);
            ..set_spacing(18);
            ..show_all();
        };
    }

    fn properties() -> &'static [glib::ParamSpec] {
        use once_cell::sync::Lazy;

        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![glib::ParamSpec::boxed(
                "colors",
                "colors",
                "colors",
                TestingColors::get_type(),
                glib::ParamFlags::READABLE,
            )]
        });

        PROPERTIES.as_ref()
    }

    fn get_property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.get_name() {
            "colors" => self.colors.borrow().to_value(),
            _ => unimplemented!(),
        }
    }
}

impl WidgetImpl for TestingInner {}
impl ContainerImpl for TestingInner {}
impl BoxImpl for TestingInner {}

glib::wrapper! {
    pub struct Testing(ObjectSubclass<TestingInner>)
        @extends gtk::Box, gtk::Container, gtk::Widget, @implements gtk::Orientable;
}

async fn import_keymap_hack(board: &Board, keymap: &backend::KeyMap) -> Result<(), String> {
    let futures = FuturesUnordered::new();
    for key in board.keys() {
        if let Some(scancodes) = keymap.map.get(&key.logical_name) {
            for layer in 0..scancodes.len() {
                futures.push(key.set_scancode(layer, &scancodes[layer]));
            }
        }
    }
    futures.try_collect::<()>().await
}

impl Testing {
    fn update_benchmarks(&self) {
        for (port_desc, port_result) in TestResults::global().bench.read().unwrap().iter() {
            if let Some(bench_label) = self.inner().bench_labels.get(port_desc) {
                match port_result {
                    Ok(ok) => {
                        bench_label.set_text(&format!("{:.2} MB/s ✅", ok));
                    }
                    Err(err) => {
                        bench_label.set_text(&format!("{} ❌", err));
                    }
                }
            } else {
                error!("{} label not found", port_desc);
            }
        }
    }

    fn connect_bench_button(&self) {
        let obj_btn = self.clone();
        self.inner().bench_button.connect_clicked(move |button| {
            button.set_label("Running USB test");

            let obj_spawn = obj_btn.clone();
            glib::MainContext::default().spawn_local(async move {
                let testing = obj_spawn.inner();

                while testing.bench_button.get_active() {
                    match testing.board.benchmark().await {
                        Ok(benchmark) => {
                            for (port_desc, port_result) in benchmark.port_results.iter() {
                                let text = format!("{:.2?}", port_result);
                                info!("{}: {}", port_desc, text);
                                if let Some(bench_result) = TestResults::global()
                                    .bench
                                    .write()
                                    .unwrap()
                                    .get_mut(port_desc.as_str())
                                {
                                    match bench_result {
                                        Ok(old) => match port_result {
                                            Ok(new) => {
                                                // Replace good results with better results
                                                if new > old {
                                                    *bench_result = Ok(*new);
                                                }
                                            }
                                            Err(_) => (),
                                        },
                                        Err(err) => {
                                            // Replace errors with newest results
                                            *bench_result = port_result.clone();
                                        }
                                    }
                                } else {
                                    error!("{} label result not found", port_desc);
                                }
                            }
                        }
                        Err(err) => {
                            let message = format!("Benchmark failed to run: {}", err);
                            error!("{}", message);
                            //TODO: have a global label?
                            for (_, bench_label) in testing.bench_labels.iter() {
                                bench_label.set_text(&message);
                            }
                        }
                    }

                    obj_spawn.update_benchmarks();

                    glib::timeout_future(std::time::Duration::new(1, 0)).await;
                }

                testing.bench_button.set_label("Run USB test");
            });
        });
    }

    fn test_buttons_sensitive(&self, test_index: usize, sensitive: bool) {
        for i in 0..3 {
            self.inner().start_buttons[i].set_sensitive(sensitive);
            self.inner().stop_buttons[i].set_sensitive(i == test_index && !sensitive);
        }
    }

    async fn selma(&self, test_index: usize, selma_kind: SelmaKind) {
        let testing = self.inner();

        info!("Disabling test buttons");
        self.test_buttons_sensitive(test_index, false);

        info!("Save and clear keymap");
        let keymap = testing.board.export_keymap();
        {
            let mut empty = keymap.clone();
            for (_name, codes) in empty.map.iter_mut() {
                for code in codes.iter_mut() {
                    *code = "NONE".to_string();
                }
            }
            if let Err(err) = import_keymap_hack(&testing.board, &empty).await {
                error!("Failed to clear keymap: {}", err);
            }
        }

        #[allow(unused_braces)]
        let (future, handle) = abortable(
            clone!(@strong self as self_ => async move { self_.selma_tests(test_index, selma_kind).await }),
        );

        let mut handles = testing.test_abort_handles.borrow_mut();
        if let Some(prev_handle) = &handles[test_index] {
            prev_handle.abort();
        }
        handles[test_index] = Some(handle);
        drop(handles);

        let _ = future.await;

        info!("Restore keymap");
        if let Err(err) = import_keymap_hack(&testing.board, &keymap).await {
            error!("Failed to restore keymap: {}", err);
        }

        info!("Enabling test buttons");
        self.test_buttons_sensitive(test_index, true);
    }

    async fn selma_tests(&self, test_index: usize, selma_kind: SelmaKind) {
        let testing = self.inner();

        let test_label = &testing.test_labels[test_index];

        for test_run in 1i32.. {
            let message = format!("Test {} running", test_run);
            info!("{}", message);
            test_label.set_text(&message);

            let selma = match testing.board.selma(selma_kind).await {
                Ok(ok) => ok,
                Err(err) => {
                    let message = format!("Test {} failed to run: {}", test_run, err);
                    error!("{}", message);
                    test_label.set_text(&message);
                    break;
                }
            };

            for row in 0..selma.max_rows() {
                for col in 0..selma.max_cols() {
                    let r = if selma.missing.get(row, col).unwrap_or(false) {
                        255
                    } else {
                        0
                    };
                    let g = if selma.sticking.get(row, col).unwrap_or(false) {
                        255
                    } else {
                        0
                    };
                    let b = if selma.bouncing.get(row, col).unwrap_or(false) {
                        255
                    } else {
                        0
                    };
                    if r != 0 || g != 0 || b != 0 {
                        testing
                            .colors
                            .borrow_mut()
                            .0
                            .insert((row, col), Rgb::new(r, g, b));
                    } else {
                        testing.colors.borrow_mut().0.remove(&(row, col));
                    }
                }
            }

            self.notify("colors");

            if selma.success() {
                let message = format!("Test {} successful", test_run);
                info!("{}", message);
                test_label.set_text(&message);
            } else {
                let message = format!("Test {} failed", test_run);
                error!("{}", message);
                test_label.set_text(&message);
                break;
            }
        }
    }

    fn connect_start_buttons(&self) {
        for i in 0..3 {
            self.inner().start_buttons[i].connect_clicked(
                clone!(@strong self as self_ => move |_| {
                    glib::MainContext::default().spawn_local(clone!(@strong self_ => async move {
                        self_.selma(i, SelmaKind::Normal).await
                    }));
                }),
            );
        }
    }

    fn connect_stop_buttons(&self) {
        for i in 0..3 {
            self.inner().stop_buttons[i].connect_clicked(
                clone!(@strong self as self_ => move |_| {
                    if let Some(handle) = self_.inner().test_abort_handles.borrow_mut()[i].take() {
                        handle.abort();
                    }
                }),
            );
        }
    }

    fn connect_reset_button(&self) {
        let obj_btn = self.clone();
        self.inner().reset_button.connect_clicked(move |_button| {
            TestResults::global().reset();
            obj_btn.update_benchmarks();
        });
    }

    pub fn new(board: Board) -> Self {
        let obj: Self = glib::Object::new(&[]).unwrap();
        obj.inner().board.set(board);
        obj.connect_bench_button();
        obj.connect_start_buttons();
        obj.connect_stop_buttons();
        obj.connect_reset_button();
        obj.update_benchmarks();
        obj
    }

    fn inner(&self) -> &TestingInner {
        TestingInner::from_instance(self)
    }
}
