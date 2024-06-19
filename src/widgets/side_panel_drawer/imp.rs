use gio::glib::{property::PropertyGet, WeakRef};
use gtk::{cairo::ImageSurface, glib, prelude::WidgetExt, subclass::prelude::*};

#[derive(Default)]
pub struct SidePanelDrawer {
    base_surface: ImageSurface,
    base_surface: ImageSurface,
}

#[glib::object_subclass]
impl ObjectSubclass for SidePanelDrawer {
    const NAME: &'static str = "MySidePanelDrawer";
    type Type = super::SidePanelDrawer;
    type ParentType = gtk::DrawingArea;
}

impl ObjectImpl for SidePanelDrawer {}
impl WidgetImpl for SidePanelDrawer {}
impl DrawingAreaImpl for SidePanelDrawer {}

impl SidePanelDrawer {
    fn t(&self) {
        self.obj().queue_draw();
    }
}
