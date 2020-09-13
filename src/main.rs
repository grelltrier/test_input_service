/*
 * Copyright (c) 2017 Boucher, Antoni <bouanto@zoho.com>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to
 * use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
 * the Software, and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
 * FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
 * COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
 * IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
 * CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */
extern crate gtk;

use gdk_sys::{GdkDisplay, GdkSeat};
use glib::translate::ToGlibPtr;
use gtk::{
    Button, ButtonExt, ContainerExt, Inhibit, Label, LabelExt, Orientation::Vertical, WidgetExt,
    Window, WindowType,
};
use input_method_service::*;
use relm::{connect, Relm, Update, Widget, WidgetTest};
use relm_derive::Msg;
use wayland_client::{
    protocol::wl_seat::WlSeat, sys::client::wl_display, Display, EventQueue, GlobalManager, Proxy,
};
use wayland_protocols::unstable::text_input::v3::client::zwp_text_input_v3::{
    ContentHint, ContentPurpose,
};
use zwp_input_method::input_method_unstable_v2::zwp_input_method_manager_v2::ZwpInputMethodManagerV2;

#[allow(non_camel_case_types)]
type wl_seat = libc::c_void;

extern "C" {
    fn gdk_wayland_display_get_wl_display(display: *mut GdkDisplay) -> *mut wl_display;
    fn gdk_wayland_seat_get_wl_seat(seat: *mut GdkSeat) -> *mut wl_seat;
}

#[derive(Clone, Debug)]
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

struct Model {
    counter: i32,
}

#[derive(Msg)]
enum Msg {
    Decrement,
    Increment,
    Quit,
}

// Create the structure that holds the widgets used in the view.
#[derive(Clone)]
struct Widgets {
    counter_label: Label,
    minus_button: Button,
    plus_button: Button,
    window: Window,
}

struct Win {
    model: Model,
    im_service: IMService<TestConnector>,
    event_queue: EventQueue,
    widgets: Widgets,
}

impl Update for Win {
    // Specify the model used for this widget.
    type Model = Model;
    // Specify the model parameter used to init the model.
    type ModelParam = ();
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(_: &Relm<Self>, _: ()) -> Model {
        Model { counter: 0 }
    }

    fn update(&mut self, event: Msg) {
        let label = &self.widgets.counter_label;

        match event {
            Msg::Decrement => {
                self.model.counter -= 1;
                // Manually update the view.
                label.set_text(&self.model.counter.to_string());
                self.im_service.commit_string("Hello world!".to_string());
                self.im_service.commit();
            }
            Msg::Increment => {
                self.model.counter += 1;
                label.set_text(&self.model.counter.to_string());
                self.event_queue
                    .dispatch(&mut (), |event, _, _| println!("Event: {:?}", event))
                    .unwrap();
            }
            Msg::Quit => gtk::main_quit(),
        }
    }
}

impl Widget for Win {
    // Specify the type of the root widget.
    type Root = Window;

    // Return the root widget.
    fn root(&self) -> Self::Root {
        self.widgets.window.clone()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        // Create the view using the normal GTK+ method calls.
        let vbox = gtk::Box::new(Vertical, 0);

        let plus_button = Button::with_label("+");
        vbox.add(&plus_button);

        let counter_label = Label::new(Some("0"));
        vbox.add(&counter_label);

        let minus_button = Button::with_label("-");
        vbox.add(&minus_button);

        let window = Window::new(WindowType::Toplevel);

        window.add(&vbox);

        let gdk_display = gdk::Display::get_default();
        let wl_display_sys =
            unsafe { gdk_wayland_display_get_wl_display(gdk_display.to_glib_none().0) };
        let wl_display = unsafe { Display::from_external_display(wl_display_sys) };

        let gdk_seat = gdk_display.expect("No gdk_display").get_default_seat(); //.expect("No gdk_seat");
        let wl_seat_sys = unsafe { gdk_wayland_seat_get_wl_seat(gdk_seat.to_glib_none().0) };
        let wl_seat = unsafe { Proxy::<WlSeat>::from_c_ptr(wl_seat_sys as *mut _) };
        let seat: WlSeat = WlSeat::from(wl_seat);

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
                |_, _, _| println!("Event received that was not handled"), // For testing
                                                                           //|_, _, _| unreachable!(), // Original
            )
            .unwrap();

        let connector = TestConnector {};
        let im_manager = get_wayland_im_manager(&global_manager);
        let im_service = IMService::new(&seat, im_manager, connector);

        /*minus_button.connect_clicked(move |_| {
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
        });*/

        let window_clone = window.clone();
        make_overlay_layer(window_clone);
        window.show_all();

        // Send the message Increment when the button is clicked.
        connect!(relm, plus_button, connect_clicked(_), Msg::Increment);
        connect!(relm, minus_button, connect_clicked(_), Msg::Decrement);
        connect!(
            relm,
            window,
            connect_delete_event(_, _),
            return (Some(Msg::Quit), Inhibit(false))
        );

        Win {
            model,
            im_service,
            event_queue, // Needs to be preserved because a new event queue is created and not gtk's reused
            widgets: Widgets {
                counter_label,
                minus_button,
                plus_button,
                window,
            },
        }
    }
}

impl WidgetTest for Win {
    type Widgets = Widgets;

    fn get_widgets(&self) -> Self::Widgets {
        self.widgets.clone()
    }
}

fn main() {
    Win::run(()).expect("Win::run failed");
}

pub fn make_overlay_layer(window: gtk::Window) {
    // Before the window is first realized, set it up to be a layer surface
    gtk_layer_shell::init_for_window(&window);

    // Order above normal windows
    gtk_layer_shell::set_layer(&window, gtk_layer_shell::Layer::Overlay);

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

fn get_wayland_im_manager(
    global_manager: &GlobalManager,
) -> wayland_client::Main<ZwpInputMethodManagerV2> {
    global_manager
        .instantiate_exact::<ZwpInputMethodManagerV2>(1)
        .expect("Error: Your compositor does not understand the virtual_keyboard protocol!")
}
