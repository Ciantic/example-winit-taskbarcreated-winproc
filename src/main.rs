// I consider this code in public domain, and require no attribution, even
// though I added MIT license
//
// Example is adapted from dicussions in here
// https://github.com/rust-windowing/winit/issues/1052

use std::error::Error;
use winapi::{
    shared::{
        basetsd::{DWORD_PTR, UINT_PTR},
        minwindef::{LPARAM, LRESULT, UINT, WPARAM},
        windef::HWND,
    },
    um::{commctrl, winuser},
};
use winit::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
    platform::windows::WindowExtWindows,
    window::WindowBuilder,
};

#[derive(Debug, Clone, Copy)]
enum AppEvent {
    WindowsTaskbarCreated,
}

type AppEventLoopProxy = EventLoopProxy<AppEvent>;

// For pattern matching in winproc it's easier to add a mod
mod msgs {
    pub const WM_USER_CREATE: u32 = 0x400 + 1;
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::<AppEvent>::with_user_event();
    let window = WindowBuilder::new().with_title("").build(&event_loop)?;

    unsafe {
        // Pass the sender in the winproc data
        let sender_box: Box<AppEventLoopProxy> = Box::new(event_loop.create_proxy());
        commctrl::SetWindowSubclass(
            window.hwnd() as HWND,
            Some(winproc),
            0,
            Box::into_raw(sender_box) as DWORD_PTR,
        );
    }

    // Top level window hwnd
    let hwnd = window.hwnd() as HWND;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::NewEvents(StartCause::Init) => {
                // Example how to send a raw message to window
                unsafe {
                    winuser::SendMessageW(hwnd, msgs::WM_USER_CREATE, 0, 0);
                }
            }

            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,

            Event::UserEvent(event) => match event {
                AppEvent::WindowsTaskbarCreated => {
                    println!("Taskbar created type-safe loop!");
                }
            },

            _ => *control_flow = ControlFlow::Wait,
        }
    });
}

// Good old winproc
unsafe extern "system" fn winproc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: UINT_PTR,
    data: DWORD_PTR,
) -> LRESULT {
    static mut WM_TASKBARCREATED: u32 = 0;

    match msg {
        winuser::WM_CREATE => {
            println!("Notice that this does not run, therefore we must use user message.");
            0
        }

        msgs::WM_USER_CREATE => {
            // Register for Taskbar created message
            let msg = "TaskbarCreated\0".encode_utf16().collect::<Vec<_>>();
            WM_TASKBARCREATED = winuser::RegisterWindowMessageW(msg.as_ptr());

            println!("Got user message in winproc!");
            0
        }

        x if x == WM_TASKBARCREATED => {
            println!("Taskbar created in winproc!");

            // Send the user message to type-safe event loop
            let proxy = &mut *(data as *mut AppEventLoopProxy);
            let _ = proxy.send_event(AppEvent::WindowsTaskbarCreated);

            0
        }

        winuser::WM_DESTROY => {
            // Clean up the event loop proxy
            Box::from_raw(data as *mut AppEventLoopProxy);
            0
        }

        _ => commctrl::DefSubclassProc(hwnd, msg, wparam, lparam),
    }
}
