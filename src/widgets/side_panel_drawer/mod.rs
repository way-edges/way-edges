mod imp;

use std::{cell::RefCell, rc::Rc};

use gio::{
    glib::{clone::Downgrade, property::PropertyGet},
    prelude::ObjectExt,
    subclass::prelude::ObjectSubclassIsExt,
};
use glib::Object;
use gtk::{
    glib,
    prelude::{DrawingAreaExtManual, WidgetExt},
};

glib::wrapper! {
    pub struct SidePanelDrawer(ObjectSubclass<imp::SidePanelDrawer>)
        @extends gtk::Widget, gtk::DrawingArea,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

struct B();
struct A(Option<B>);

impl SidePanelDrawer {
    pub fn new() -> Self {
        let s: Self = Object::builder().build();
        let a = Rc::new(RefCell::new(A(None)));
        set_draw_func(move || a.borrow_mut().0 = Some(B()));
        // s.set_draw_func(move |obj, ctx, w, h| {
        //     a.borrow_mut().0 = Some(1);
        // });
        // s.set_draw_func(glib::clone!(@weak A=>a move||))
        s
    }
}

impl Default for SidePanelDrawer {
    fn default() -> Self {
        Self::new()
    }
}

pub fn set_draw_func<P>(mut draw_func: P)
where
    P: FnMut() + 'static,
{
    loop {
        draw_func();
    }
}
