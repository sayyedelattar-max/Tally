//! A small, dynamically loaded binding to libmpv.
//!
//! The library is opened at run time (`libmpv-2.dll` next to the app on Windows), so the app builds
//! and runs without it; when it is missing, [`Mpv::load`] simply fails and the caller falls back to
//! the built-in player.

use std::ffi::{c_char, c_double, c_int, c_void, CStr, CString};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use libloading::Library;

use crate::error::{Error, Result};

type Handle = *mut c_void;

#[repr(C)]
struct RawEvent {
    id: c_int,
    error: c_int,
    reply_userdata: u64,
    data: *mut c_void,
}

#[repr(C)]
struct RawClientMessage {
    num_args: c_int,
    args: *mut *const c_char,
}

#[repr(C)]
struct RawEndFile {
    reason: c_int,
    error: c_int,
}

struct Api {
    _lib: Library,
    create: unsafe extern "C" fn() -> Handle,
    initialize: unsafe extern "C" fn(Handle) -> c_int,
    set_option_string: unsafe extern "C" fn(Handle, *const c_char, *const c_char) -> c_int,
    set_property_string: unsafe extern "C" fn(Handle, *const c_char, *const c_char) -> c_int,
    get_property_string: unsafe extern "C" fn(Handle, *const c_char) -> *mut c_char,
    command: unsafe extern "C" fn(Handle, *mut *const c_char) -> c_int,
    wait_event: unsafe extern "C" fn(Handle, c_double) -> *mut RawEvent,
    terminate_destroy: unsafe extern "C" fn(Handle),
    free: unsafe extern "C" fn(*mut c_void),
    error_string: unsafe extern "C" fn(c_int) -> *const c_char,
}

/// What the player tells us.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Nothing happened within the wait.
    None,
    /// The player is going away (window closed, `quit`).
    Shutdown,
    FileLoaded,
    /// A file finished: `eof` is a normal end, `error` a failure (with the player's wording).
    EndFile { eof: bool, error: Option<String> },
    /// `script-message` from a key binding: the arguments.
    Message(Vec<String>),
    Other,
}

pub struct Mpv {
    api: Arc<Api>,
    h: Handle,
}

// libmpv's client API is thread-safe.
unsafe impl Send for Mpv {}
unsafe impl Sync for Mpv {}

fn c(s: &str) -> Result<CString> {
    CString::new(s).map_err(|_| Error::Account("text contained a NUL byte".into()))
}

impl Mpv {
    /// Places where the library may live, best first.
    pub fn candidates(extra_dirs: &[PathBuf]) -> Vec<PathBuf> {
        let names: &[&str] = if cfg!(windows) {
            &["libmpv-2.dll", "mpv-2.dll", "libmpv.dll"]
        } else if cfg!(target_os = "macos") {
            &["libmpv.2.dylib", "libmpv.dylib"]
        } else {
            &["libmpv.so.2", "libmpv.so.1", "libmpv.so"]
        };
        let mut out = Vec::new();
        for d in extra_dirs {
            for n in names {
                out.push(d.join(n));
            }
        }
        // plain names let the system search its usual places
        out.extend(names.iter().map(PathBuf::from));
        out
    }

    pub fn load(extra_dirs: &[PathBuf]) -> Result<Api0> {
        let mut last = String::from("libmpv was not found");
        for p in Self::candidates(extra_dirs) {
            if p.is_absolute() && !p.exists() {
                continue;
            }
            match open(&p) {
                Ok(api) => return Ok(Api0(Arc::new(api))),
                Err(e) => last = e,
            }
        }
        Err(Error::Account(last))
    }
}

/// A loaded library, from which players are created.
#[derive(Clone)]
pub struct Api0(Arc<Api>);

impl Api0 {
    pub fn create(&self) -> Result<Mpv> {
        let h = unsafe { (self.0.create)() };
        if h.is_null() {
            return Err(Error::Account("the player could not be created".into()));
        }
        Ok(Mpv { api: self.0.clone(), h })
    }
}

fn open(path: &Path) -> std::result::Result<Api, String> {
    unsafe {
        let lib = Library::new(path).map_err(|e| format!("{}: {e}", path.display()))?;
        macro_rules! sym {
            ($n:literal) => {
                *lib.get($n).map_err(|e| format!("libmpv is missing {}: {e}", String::from_utf8_lossy($n)))?
            };
        }
        Ok(Api {
            create: sym!(b"mpv_create\0"),
            initialize: sym!(b"mpv_initialize\0"),
            set_option_string: sym!(b"mpv_set_option_string\0"),
            set_property_string: sym!(b"mpv_set_property_string\0"),
            get_property_string: sym!(b"mpv_get_property_string\0"),
            command: sym!(b"mpv_command\0"),
            wait_event: sym!(b"mpv_wait_event\0"),
            terminate_destroy: sym!(b"mpv_terminate_destroy\0"),
            free: sym!(b"mpv_free\0"),
            error_string: sym!(b"mpv_error_string\0"),
            _lib: lib,
        })
    }
}

impl Mpv {
    fn err(&self, code: c_int) -> Error {
        let s = unsafe { CStr::from_ptr((self.api.error_string)(code)) }.to_string_lossy().into_owned();
        Error::Account(format!("player: {s}"))
    }

    /// Settings that must be made before [`Mpv::init`].
    pub fn option(&self, name: &str, value: &str) -> Result<()> {
        let (n, v) = (c(name)?, c(value)?);
        let r = unsafe { (self.api.set_option_string)(self.h, n.as_ptr(), v.as_ptr()) };
        if r < 0 { Err(self.err(r)) } else { Ok(()) }
    }

    pub fn init(&self) -> Result<()> {
        let r = unsafe { (self.api.initialize)(self.h) };
        if r < 0 { Err(self.err(r)) } else { Ok(()) }
    }

    pub fn set(&self, name: &str, value: &str) -> Result<()> {
        let (n, v) = (c(name)?, c(value)?);
        let r = unsafe { (self.api.set_property_string)(self.h, n.as_ptr(), v.as_ptr()) };
        if r < 0 { Err(self.err(r)) } else { Ok(()) }
    }

    pub fn get(&self, name: &str) -> Option<String> {
        let n = c(name).ok()?;
        let p = unsafe { (self.api.get_property_string)(self.h, n.as_ptr()) };
        if p.is_null() {
            return None;
        }
        let s = unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned();
        unsafe { (self.api.free)(p as *mut c_void) };
        Some(s)
    }

    pub fn command(&self, args: &[&str]) -> Result<()> {
        let owned: Vec<CString> = args.iter().map(|a| c(a)).collect::<Result<_>>()?;
        let mut ptrs: Vec<*const c_char> = owned.iter().map(|s| s.as_ptr()).collect();
        ptrs.push(std::ptr::null());
        let r = unsafe { (self.api.command)(self.h, ptrs.as_mut_ptr()) };
        if r < 0 { Err(self.err(r)) } else { Ok(()) }
    }

    /// Wait up to `secs` for the next event.
    pub fn wait(&self, secs: f64) -> Event {
        let ev = unsafe { &*(self.api.wait_event)(self.h, secs) };
        match ev.id {
            0 => Event::None,
            1 => Event::Shutdown,
            8 => Event::FileLoaded,
            7 => {
                let end = unsafe { ev.data.cast::<RawEndFile>().as_ref() };
                match end {
                    Some(e) if e.reason == 0 => Event::EndFile { eof: true, error: None },
                    Some(e) if e.reason == 4 => {
                        let msg = unsafe { CStr::from_ptr((self.api.error_string)(e.error)) }.to_string_lossy().into_owned();
                        Event::EndFile { eof: false, error: Some(msg) }
                    }
                    _ => Event::Other, // stopped by us (loading something else) or quit
                }
            }
            16 => {
                let m = unsafe { ev.data.cast::<RawClientMessage>().as_ref() };
                let mut out = Vec::new();
                if let Some(m) = m {
                    for i in 0..m.num_args.max(0) as usize {
                        let p = unsafe { *m.args.add(i) };
                        if !p.is_null() {
                            out.push(unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned());
                        }
                    }
                }
                Event::Message(out)
            }
            _ => Event::Other,
        }
    }
}

impl Drop for Mpv {
    fn drop(&mut self) {
        unsafe { (self.api.terminate_destroy)(self.h) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(path: &Path) {
        // 0.3 s of silence, 8-bit mono 8 kHz
        let data = vec![128u8; 2400];
        let mut f = Vec::new();
        f.extend(b"RIFF");
        f.extend(((36 + data.len()) as u32).to_le_bytes());
        f.extend(b"WAVEfmt ");
        f.extend(16u32.to_le_bytes());
        f.extend(1u16.to_le_bytes());
        f.extend(1u16.to_le_bytes());
        f.extend(8000u32.to_le_bytes());
        f.extend(8000u32.to_le_bytes());
        f.extend(1u16.to_le_bytes());
        f.extend(8u16.to_le_bytes());
        f.extend(b"data");
        f.extend((data.len() as u32).to_le_bytes());
        f.extend(&data);
        std::fs::write(path, f).unwrap();
    }

    /// Runs only where libmpv is installed (it is on the CI machines that build the app for Windows).
    #[test]
    fn plays_a_file_and_reports_events() {
        let Ok(api) = Mpv::load(&[]) else {
            eprintln!("libmpv not installed; skipping");
            return;
        };
        let dir = std::env::temp_dir().join(format!("tally-mpv-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("tone.wav");
        wav(&file);
        let m = api.create().unwrap();
        for (k, v) in [("vo", "null"), ("ao", "null"), ("config", "no"), ("idle", "yes"), ("terminal", "no")] {
            m.option(k, v).unwrap();
        }
        m.init().unwrap();
        m.command(&["script-message", "tally-next", "x"]).unwrap();
        m.command(&["loadfile", file.to_str().unwrap()]).unwrap();
        let (mut loaded, mut ended, mut msg) = (false, false, false);
        for _ in 0..40 {
            match m.wait(0.25) {
                Event::FileLoaded => loaded = true,
                Event::EndFile { eof: true, .. } => {
                    ended = true;
                    break;
                }
                Event::Message(a) if a == ["tally-next", "x"] => msg = true,
                _ => {}
            }
        }
        assert!(loaded && ended, "loaded={loaded} ended={ended}");
        assert!(msg, "script-message should reach this client");
        assert_eq!(m.get("pause").as_deref(), Some("no"));
        // a missing file is reported as an error, not a crash
        m.command(&["loadfile", dir.join("nope.wav").to_str().unwrap()]).unwrap();
        let mut failed = false;
        for _ in 0..20 {
            if let Event::EndFile { error: Some(_), .. } = m.wait(0.25) {
                failed = true;
                break;
            }
        }
        assert!(failed);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
