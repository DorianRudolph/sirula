use gtk::glib;
use gtk::subclass::prelude::*;

use crate::gio::AppInfo;
use gtk::Label;

#[derive(Default)]
pub struct AppRow {
    pub display_string: String,
    pub search_string: String,
    pub extra_range: Option<(u32, u32)>,
    //pub info: AppInfo,
    pub label: Label,
    pub score: i64,
    //pub history: HistoryData,
44

#[glib::object_subclass]
impl ObjectSubclass for AppRow {
    const NAME: &'static str = "AppRow";
    type Type = super::AppRow;
    type ParentType = gtk::ListBoxRow;
}

impl ObjectImpl for AppRow {}
impl WidgetImpl for AppRow {}
impl ListBoxRowImpl for AppRow {}
