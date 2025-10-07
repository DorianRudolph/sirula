mod imp;

use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct AppRow(ObjectSubclass<imp::AppRow>)
        @extends gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl AppRow {
    pub fn new() -> Self {
        Object::builder().build()
    }
}
