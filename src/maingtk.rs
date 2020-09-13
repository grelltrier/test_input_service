extern crate gtk;

use gio::prelude::*;
use gtk::prelude::*;
use gtk::WindowType;
use input_method_service::*;
use std::env::args;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Display, GlobalManager, Main};
use wayland_protocols::unstable::text_input::v3::client::zwp_text_input_v3::{
    ContentHint, ContentPurpose,
};
use zwp_input_method::input_method_unstable_v2::zwp_input_method_manager_v2::ZwpInputMethodManagerV2;

struct TestConnector {}

impl KeyboardVisability for TestConnector {
    fn show_keyboard(&self) {
        println!("Show keyboard");
    }
    fn hide_keyboard(&self) {
        println!("Hide keyboard");
    }
}

impl HintPurpose for TestConnector {
    fn set_hint_purpose(&self, content_hint: ContentHint, content_purpose: ContentPurpose) {
        println!("Hint: {:?}, Purpose: {:?}", content_hint, content_purpose);
    }
}

fn main() {
    let application =
        gtk::Application::new(Some("com.github.gtk-rs.examples.basic"), Default::default())
            .expect("Initialization failed...");

    application.connect_activate(|app| {
        build_ui(app);
    });

    application.run(&args().collect::<Vec<_>>());
}

fn build_ui(application: &gtk::Application) {
    //let application_window = gtk::ApplicationWindow::new(application);

    //application_window.set_title("First GTK+ Program");
    //application_window.set_border_width(10);
    //application_window.set_position(gtk::WindowPosition::Center);
    //application_window.set_default_size(350, 70);
    let connector = TestConnector {};
    let (_display, seat, global_manager) = get_wayland_display_seat_globalmgr();
    let button = gtk::Button::with_label("Click me!");
    let im_manager = get_wayland_im_manager(&global_manager);
    let im_service = IMService::new(&seat, im_manager, connector);
    button.connect_clicked(move |_| {
        println!("im_service is active: {}", im_service.is_active());
        let commit_string_result = im_service.commit_string(String::from("HelloWorld"));
        match commit_string_result {
            Ok(()) => println!("Successfully committed!"),
            _ => println!("Error when committing!"),
        }
        let commit_result = im_service.commit();
        match commit_result {
            Ok(()) => println!("Successfully committed!"),
            _ => println!("Error when committing!"),
        }
    });
    let window = gtk::Window::new(WindowType::Toplevel);
    window.add(&button);
    let window_clone = window.clone();
    make_overlay_layer(window_clone);
    //application_window.add(&window);
    window.show_all();
}

fn get_wayland_display_seat_globalmgr() -> (Display, Main<WlSeat>, GlobalManager) {
    let display = Display::connect_to_name("wayland-0").unwrap();
    let mut event_queue = display.create_event_queue();
    let attached_display = (*display).clone().attach(event_queue.token());

    let global_manager = GlobalManager::new(&attached_display);

    // Make a synchronized roundtrip to the wayland server.
    //
    // When this returns it must be true that the server has already
    // sent us all available globals.
    event_queue
        .sync_roundtrip(&mut (), |_, _, _| unreachable!())
        .unwrap();
    let seat = global_manager.instantiate_exact::<WlSeat>(1).unwrap();
    (display, seat, global_manager)
}

pub fn make_overlay_layer(window: gtk::Window) {
    // Before the window is first realized, set it up to be a layer surface
    gtk_layer_shell::init_for_window(&window);

    // Order above normal windows
    gtk_layer_shell::set_layer(&window, gtk_layer_shell::Layer::Overlay);

    // Push other windows out of the way
    //gtk_layer_shell::auto_exclusive_zone_enable(&window);

    // The margins are the gaps around the window's edges
    // Margins and anchors can be set like this...
    gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Left, 0);
    gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Right, 0);
    //gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Top, 20);
    // ... or like this
    // Anchors are if the window is pinned to each edge of the output
    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Left, true);
    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Right, true);
    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Top, false);
    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Bottom, true);
}

/*fn get_wayland_global_manager(
    wl_display: &Display,
    seat: &Main<WlSeat>,
) -> (GlobalManager, Proxy<WlSeat>) {
    //let gdk_seat = gdk_display.expect("No gdk_display").get_default_seat(); //.expect("No gdk_seat");
    //let wl_seat_sys = unsafe { gdk_wayland_seat_get_wl_seat(gdk_seat.to_glib_none().0) };
    //let wl_seat = unsafe { Proxy::<WlSeat>::from_c_ptr(wl_seat_sys as *mut _) };

    // Create the event queue
    let mut event_queue = wl_display.create_event_queue();
    // Attach the display
    let attached_display = wl_display.attach(event_queue.token());

    let global_manager = GlobalManager::new(&attached_display);

    // sync_roundtrip is a special kind of dispatching for the event queue.
    // Rather than just blocking once waiting for replies, it'll block
    // in a loop until the server has signalled that it has processed and
    // replied accordingly to all requests previously sent by the client.
    //
    // In our case, this allows us to be sure that after this call returns,
    // we have received the full list of globals.
    event_queue
        .sync_roundtrip(
            // we don't use a global state for this example
            &mut (),
            // The only object that can receive events is the WlRegistry, and the
            // GlobalManager already takes care of assigning it to a callback, so
            // we cannot receive orphan events at this point
            |_, _, _| unreachable!(),
        )
        .unwrap();
    (global_manager, wl_seat)
}*/

fn get_wayland_im_manager(
    global_manager: &GlobalManager,
) -> wayland_client::Main<ZwpInputMethodManagerV2> {
    global_manager
        .instantiate_exact::<ZwpInputMethodManagerV2>(1)
        .expect("Error: Your compositor does not understand the virtual_keyboard protocol!")
}
