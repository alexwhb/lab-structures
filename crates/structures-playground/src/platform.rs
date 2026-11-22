//! OS glue: truss netlists the system asks the app to open (NAV-6).
//!
//! `packaging/Info.plist.in` registers `.str`, so Finder's "Open With", a drop on the Dock icon
//! and `open -a "Structures Playground" river-crossing.str` hand files to the app as the Apple
//! Event `odoc` (also for the documents of the launch itself); `structures-playground
//! river-crossing.str` passes them on the command line. Both land in one queue; the app drains
//! it at the start of every frame ([`deliver`]): each netlist is read and handed to the
//! Advanced workbench as a [`Carry::Netlist`](crate::Carry::Netlist) (the screen parses it and
//! shows any errors on their lines), the app switches to Advanced and logs `opened netlist
//! <path>`; a file that can't be read shows a toast instead.
//!
//! Only the app registered with [`set_receiver`] takes queued documents (the real app's
//! window; a test registers its harness), so other apps in the same process — parallel tests,
//! the smoke run — never see them.

use crate::app::StructuresApp;
use crate::router::Route;
use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Documents the system asked us to open, until the receiving app takes them.
static OPENED: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
/// The context of the app that takes them (woken when a document arrives while it idles).
static RECEIVER: Mutex<Option<egui::Context>> = Mutex::new(None);

/// The file extensions the app opens (`packaging/Info.plist.in`'s document types).
pub const EXTENSIONS: [&str; 1] = ["str"];

/// Queues documents to open and wakes the receiving app.
pub fn queue_open(paths: impl IntoIterator<Item = PathBuf>) {
    if let Ok(mut q) = OPENED.lock() {
        q.extend(paths);
    }
    if let Ok(r) = RECEIVER.lock()
        && let Some(ctx) = r.as_ref()
    {
        ctx.request_repaint();
    }
}

/// Makes the app drawn with `ctx` the one that takes queued documents.
pub fn set_receiver(ctx: &egui::Context) {
    if let Ok(mut r) = RECEIVER.lock() {
        *r = Some(ctx.clone());
    }
    // documents may have arrived before the window existed (the launch's own `odoc`)
    ctx.request_repaint();
}

/// The queued documents (drained) if `ctx` is the receiver; nothing otherwise.
pub fn take_opened(ctx: &egui::Context) -> Vec<PathBuf> {
    let is_receiver = RECEIVER
        .lock()
        .is_ok_and(|r| r.as_ref().is_some_and(|c| c == ctx));
    if !is_receiver {
        return Vec::new();
    }
    OPENED
        .lock()
        .map(|mut q| std::mem::take(&mut *q))
        .unwrap_or_default()
}

/// Opens every queued document in `app` if `ctx` is the receiver (the app calls this first
/// thing every frame).
pub fn deliver(app: &mut StructuresApp, ctx: &egui::Context) {
    for path in take_opened(ctx) {
        open_netlist(app, &path);
    }
}

/// Opens a `.str` netlist: hands its text to Advanced (with the file stem as the document
/// name) and switches there, or toasts why it couldn't be read. Returns whether it was read.
pub fn open_netlist(app: &mut StructuresApp, path: &Path) -> bool {
    match std::fs::read_to_string(path) {
        Ok(text) => {
            log::info!("opened netlist {}", path.display());
            let name = path.file_stem().map(|s| s.to_string_lossy().into_owned());
            app.navigate_with(
                Route::Advanced,
                crate::Carry::Netlist {
                    text,
                    name,
                    from: None,
                },
            );
            true
        }
        Err(e) => {
            log::warn!("couldn't open {}: {e}", path.display());
            let file = path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            app.toast(format!("Couldn't open {file}: {e}"));
            false
        }
    }
}

/// The netlists named on a command line (`structures-playground bridge.str`): arguments with
/// a netlist extension, made absolute. Flags (and the old `-psn_…` process serial number) and
/// other arguments are not documents.
pub fn cli_paths(args: impl IntoIterator<Item = std::ffi::OsString>) -> Vec<PathBuf> {
    args.into_iter()
        .filter(|a| !a.to_string_lossy().starts_with('-'))
        .map(PathBuf::from)
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| EXTENSIONS.iter().any(|x| e.eq_ignore_ascii_case(x)))
        })
        .map(|p| std::path::absolute(&p).unwrap_or(p))
        .collect()
}

/// Before the event loop starts: queue the netlists named on the command line and listen for
/// the system's open-documents requests (macOS Apple Event `odoc`).
pub fn install_open_documents() {
    queue_open(cli_paths(std::env::args_os().skip(1)));
    #[cfg(target_os = "macos")]
    open_documents::install();
}

/// The `odoc` handler (as Breadbox's `platform.rs`).
#[cfg(target_os = "macos")]
mod open_documents {
    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::runtime::NSObject;
    use objc2::{AllocAnyThread, define_class, msg_send, sel};
    use objc2_foundation::{
        NSAppleEventDescriptor, NSAppleEventManager, NSNotification, NSNotificationCenter, NSString,
    };
    use std::ptr::NonNull;

    const CORE_EVENT_CLASS: u32 = u32::from_be_bytes(*b"aevt");
    const OPEN_DOCUMENTS: u32 = u32::from_be_bytes(*b"odoc");
    const DIRECT_OBJECT: u32 = u32::from_be_bytes(*b"----");

    define_class!(
        #[unsafe(super(NSObject))]
        #[name = "StructuresPlaygroundOpenDocumentsHandler"]
        struct Handler;

        impl Handler {
            #[unsafe(method(handleOpenDocuments:withReplyEvent:))]
            fn handle(&self, event: &NSAppleEventDescriptor, _reply: &NSAppleEventDescriptor) {
                // SAFETY: `paramDescriptorForKeyword:` takes an AEKeyword (u32)
                let list: Option<Retained<NSAppleEventDescriptor>> =
                    unsafe { msg_send![event, paramDescriptorForKeyword: DIRECT_OBJECT] };
                let Some(list) = list else { return };
                let paths: Vec<std::path::PathBuf> = (1..=list.numberOfItems())
                    .filter_map(|i| list.descriptorAtIndex(i))
                    .filter_map(|d| d.fileURLValue())
                    .filter_map(|u| u.to_file_path())
                    .collect();
                for p in &paths {
                    log::info!("asked to open {}", p.display());
                }
                super::queue_open(paths);
            }
        }
    );

    /// NSApplication installs its own `odoc` handler while it finishes launching; ours goes in
    /// right after (`NSApplicationWillFinishLaunchingNotification`), before the launch's own
    /// documents are delivered.
    pub fn install() {
        let block = RcBlock::new(|_n: NonNull<NSNotification>| {
            let handler: Retained<Handler> = unsafe { msg_send![Handler::alloc(), init] };
            let manager = NSAppleEventManager::sharedAppleEventManager();
            // SAFETY: a live handler object with the selector's signature; event class and id
            // are FourCharCodes
            unsafe {
                let _: () = msg_send![
                    &*manager,
                    setEventHandler: &*handler,
                    andSelector: sel!(handleOpenDocuments:withReplyEvent:),
                    forEventClass: CORE_EVENT_CLASS,
                    andEventID: OPEN_DOCUMENTS
                ];
            }
            // the event manager does not retain its handlers
            std::mem::forget(handler);
        });
        let name = NSString::from_str("NSApplicationWillFinishLaunchingNotification");
        let center = NSNotificationCenter::defaultCenter();
        // SAFETY: the block only touches the main-thread event manager, posted on the main thread
        let observer = unsafe {
            center.addObserverForName_object_queue_usingBlock(Some(&name), None, None, &block)
        };
        std::mem::forget(observer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn command_line_netlists() {
        let args = [
            "river-crossing.str",
            "--smoke",
            "3",
            "-psn_0_12345",
            "a/b.STR",
            "notes.txt",
        ]
        .map(OsString::from);
        let paths = cli_paths(args);
        assert_eq!(paths.len(), 2, "{paths:?}");
        assert!(paths.iter().all(|p| p.is_absolute()));
        assert!(paths[0].ends_with("river-crossing.str"));
        assert!(paths[1].ends_with("a/b.STR"));
    }

    /// Only the receiver takes the queue (this is the only unit test touching the process-wide
    /// queue).
    #[test]
    fn only_the_receiver_takes_queued_documents() {
        let (receiver, other) = (egui::Context::default(), egui::Context::default());
        set_receiver(&receiver);
        let a = PathBuf::from("/tmp/a.str");
        queue_open([a.clone()]);
        assert!(take_opened(&other).is_empty());
        assert_eq!(take_opened(&receiver), vec![a]);
        assert!(take_opened(&receiver).is_empty());
    }
}
