// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};

use crate::application::MomentumApplication;
use crate::config::{APP_ID, PROFILE};

mod imp {
    use super::*;

    #[derive(Debug, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/danhart/Momentum/ui/window.ui")]
    pub struct MomentumWindow {
        #[template_child]
        pub split_view: TemplateChild<adw::NavigationSplitView>,
        #[template_child]
        pub sidebar_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub content_page: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub status_page: TemplateChild<adw::StatusPage>,
        pub settings: gio::Settings,
    }

    impl Default for MomentumWindow {
        fn default() -> Self {
            Self {
                split_view: TemplateChild::default(),
                sidebar_list: TemplateChild::default(),
                content_page: TemplateChild::default(),
                status_page: TemplateChild::default(),
                settings: gio::Settings::new(*APP_ID),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MomentumWindow {
        const NAME: &'static str = "MomentumWindow";
        type Type = super::MomentumWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MomentumWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            if *PROFILE == "Devel" {
                obj.add_css_class("devel");
            }

            obj.load_window_size();
            obj.setup_sidebar();
        }
    }

    impl WidgetImpl for MomentumWindow {}
    impl WindowImpl for MomentumWindow {
        fn close_request(&self) -> glib::Propagation {
            if let Err(err) = self.obj().save_window_size() {
                tracing::warn!("Failed to save window state, {}", &err);
            }
            self.parent_close_request()
        }
    }
    impl ApplicationWindowImpl for MomentumWindow {}
    impl AdwApplicationWindowImpl for MomentumWindow {}
}

glib::wrapper! {
    pub struct MomentumWindow(ObjectSubclass<imp::MomentumWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionMap, gio::ActionGroup,
                    gtk::Root, gtk::Native, gtk::ShortcutManager,
                    gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl MomentumWindow {
    pub fn new(app: &MomentumApplication) -> Self {
        glib::Object::builder().property("application", app).build()
    }

    /// Wire the placeholder sidebar: selecting a row retitles the content
    /// page and, when collapsed, navigates to it.
    fn setup_sidebar(&self) {
        let imp = self.imp();
        imp.sidebar_list.connect_row_selected(glib::clone!(
            #[weak(rename_to = win)]
            self,
            move |_, row| {
                let Some(row) = row else { return };
                let Some(row) = row.downcast_ref::<adw::ActionRow>() else { return };
                let imp = win.imp();
                imp.content_page.set_title(&row.title());
                imp.status_page.set_title(&row.title());
                imp.split_view.set_show_content(true);
            }
        ));
        if let Some(first) = imp.sidebar_list.row_at_index(0) {
            imp.sidebar_list.select_row(Some(&first));
        }
    }

    fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let imp = self.imp();
        let (width, height) = self.default_size();
        imp.settings.set_int("window-width", width)?;
        imp.settings.set_int("window-height", height)?;
        imp.settings
            .set_boolean("is-maximized", self.is_maximized())?;
        Ok(())
    }

    fn load_window_size(&self) {
        let imp = self.imp();
        let width = imp.settings.int("window-width");
        let height = imp.settings.int("window-height");
        let is_maximized = imp.settings.boolean("is-maximized");
        self.set_default_size(width, height);
        if is_maximized {
            self.maximize();
        }
    }
}
