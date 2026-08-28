//! Talking to a PC/SC smart-card reader — without linking against one.
//!
//! The three implementations of this API (WinSCard on Windows, the PCSC
//! framework on macOS, pcsc-lite everywhere else) are all C libraries
//! with the same entry points, and the usual Rust bindings link against
//! them at build time. That is exactly what this module refuses to do:
//! a binary linked against `libpcsclite.so.1` does not start at all on a
//! machine that has no card middleware installed — the loader fails
//! before `main`, and the operator sees nothing but a window that never
//! opens. A pharmacy without a reader is the common case, not the
//! exception, and it must keep the application it had.
//!
//! So the library is opened by name at the moment a card is asked for.
//! No reader, no middleware, no daemon: one French sentence, and every
//! other feature untouched. It also means the build needs no system
//! package — not on a contributor's machine, not in CI, not on the
//! release runners.
//!
//! The types below are the ones the C headers use on each platform, and
//! they are not interchangeable: `DWORD` is a 64-bit `unsigned long` on
//! Linux and a 32-bit one on macOS, and a context handle is a pointer on
//! Windows and a `long` elsewhere. Getting that wrong corrupts memory
//! rather than failing, which is why they are spelled out here exactly as
//! the headers spell them.

#![allow(clippy::upper_case_acronyms)]

use std::ffi::{c_char, c_void, CString};
use std::sync::OnceLock;

#[cfg(not(target_os = "macos"))]
pub type Dword = std::ffi::c_ulong;
#[cfg(not(target_os = "macos"))]
pub type Long = std::ffi::c_long;

#[cfg(target_os = "macos")]
pub type Dword = u32;
#[cfg(target_os = "macos")]
pub type Long = i32;

#[cfg(target_os = "windows")]
type Handle = usize;
#[cfg(not(target_os = "windows"))]
type Handle = Long;

const SCOPE_USER: Dword = 0x0000;
const SHARE_SHARED: Dword = 0x0002;
const PROTOCOL_ANY: Dword = 0x0001 | 0x0002;
const LEAVE_CARD: Dword = 0x0000;
const SUCCESS: Long = 0;

/// The protocol header that precedes an APDU. The C libraries export it
/// as a global (`g_rgSCardT1Pci`), but its contents are simply the
/// protocol in force and its own size, so it is built here rather than
/// fetched as a data symbol — one less thing to look up, and one less
/// way for a library to disappoint us halfway through.
#[cfg_attr(not(target_os = "macos"), repr(C))]
#[cfg_attr(target_os = "macos", repr(C, packed))]
struct IoRequest {
    protocol: Dword,
    pci_length: Dword,
}

type FnEstablish =
    unsafe extern "system" fn(Dword, *const c_void, *const c_void, *mut Handle) -> Long;
type FnRelease = unsafe extern "system" fn(Handle) -> Long;
type FnListReaders =
    unsafe extern "system" fn(Handle, *const c_char, *mut c_char, *mut Dword) -> Long;
type FnConnect =
    unsafe extern "system" fn(Handle, *const c_char, Dword, Dword, *mut Handle, *mut Dword) -> Long;
type FnStatus = unsafe extern "system" fn(
    Handle,
    *mut c_char,
    *mut Dword,
    *mut Dword,
    *mut Dword,
    *mut u8,
    *mut Dword,
) -> Long;
type FnTransmit = unsafe extern "system" fn(
    Handle,
    *const IoRequest,
    *const u8,
    Dword,
    *mut IoRequest,
    *mut u8,
    *mut Dword,
) -> Long;
type FnDisconnect = unsafe extern "system" fn(Handle, Dword) -> Long;

/// The library, held open for the life of the process, and the seven
/// entry points this module uses.
struct Lib {
    _lib: libloading::Library,
    establish: FnEstablish,
    release: FnRelease,
    list_readers: FnListReaders,
    connect: FnConnect,
    status: FnStatus,
    transmit: FnTransmit,
    disconnect: FnDisconnect,
}

/// Where each platform keeps its implementation. The `.so.1` is tried
/// before the bare `.so` on purpose: the second is part of the
/// development package and is absent from an ordinary installation.
#[cfg(target_os = "windows")]
const CANDIDATES: &[&str] = &["WinSCard.dll"];
#[cfg(target_os = "macos")]
const CANDIDATES: &[&str] = &["/System/Library/Frameworks/PCSC.framework/PCSC"];
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
const CANDIDATES: &[&str] = &["libpcsclite.so.1", "libpcsclite.so"];

/// Windows decorates the entry points that take text with an `A`; the
/// others are named the same everywhere.
#[cfg(target_os = "windows")]
const ASCII: &str = "A";
#[cfg(not(target_os = "windows"))]
const ASCII: &str = "";

fn load() -> Result<Lib, String> {
    let mut last = String::new();
    for name in CANDIDATES {
        // SAFETY: opening a shared library runs its initialisers. These
        // are the system's own card libraries, named absolutely or by
        // soname, not a path the user supplies.
        match unsafe { libloading::Library::new(name) } {
            Ok(lib) => {
                let get = |symbol: String| -> Result<*const (), String> {
                    // SAFETY: the symbol is looked up by name and its
                    // type is the one the C header declares, spelled out
                    // above. A missing symbol is an error, not a crash.
                    unsafe {
                        lib.get::<*const ()>(symbol.as_bytes())
                            .map(|s| *s)
                            .map_err(|_| format!("point d'entrée {symbol} introuvable"))
                    }
                };
                // The names carry their terminating NUL: libloading takes
                // the bytes as given, and a name without it is looked up
                // with whatever follows in memory.
                let establish = get("SCardEstablishContext\0".to_owned())?;
                let release = get("SCardReleaseContext\0".to_owned())?;
                let list_readers = get(format!("SCardListReaders{ASCII}\0"))?;
                let connect = get(format!("SCardConnect{ASCII}\0"))?;
                let status = get(format!("SCardStatus{ASCII}\0"))?;
                let transmit = get("SCardTransmit\0".to_owned())?;
                let disconnect = get("SCardDisconnect\0".to_owned())?;
                // SAFETY: each pointer is the address of the C function
                // whose signature the type spells out.
                return Ok(unsafe {
                    Lib {
                        establish: std::mem::transmute::<*const (), FnEstablish>(establish),
                        release: std::mem::transmute::<*const (), FnRelease>(release),
                        list_readers: std::mem::transmute::<*const (), FnListReaders>(list_readers),
                        connect: std::mem::transmute::<*const (), FnConnect>(connect),
                        status: std::mem::transmute::<*const (), FnStatus>(status),
                        transmit: std::mem::transmute::<*const (), FnTransmit>(transmit),
                        disconnect: std::mem::transmute::<*const (), FnDisconnect>(disconnect),
                        _lib: lib,
                    }
                });
            }
            Err(e) => last = e.to_string(),
        }
    }
    Err(format!(
        "aucune bibliothèque PC/SC sur ce poste ({}) : {last}",
        CANDIDATES.join(", ")
    ))
}

fn lib() -> Result<&'static Lib, String> {
    static LIB: OnceLock<Result<Lib, String>> = OnceLock::new();
    LIB.get_or_init(load).as_ref().map_err(|e| e.clone())
}

/// The low thirty-two bits of a return code, so the same constant
/// matches on every platform: `SCARD_E_NO_SERVICE` is a positive
/// `0x8010001D` where `LONG` is 64 bits wide, and the same value read as
/// a negative number where it is 32.
fn code_of(code: Long) -> u32 {
    // Signed to wider unsigned sign-extends, which is what makes the
    // 32-bit and the 64-bit forms of the same code meet here.
    (code as u64 & 0xFFFF_FFFF) as u32
}

/// What a return code means, in the words an operator can act on.
///
/// The five named here are the five that actually happen at a counter:
/// the daemon is not running, no reader is plugged in, the reader is
/// empty, the card was pulled out mid-read, or the officine's own billing
/// software is holding the card. Anything else is reported by its code,
/// which is what a support call needs.
pub fn message(code: Long) -> String {
    match code_of(code) {
        0x0000_0000 => "succès".to_owned(),
        0x8010_001D | 0x8010_001E => {
            "le service de cartes n'est pas démarré sur ce poste".to_owned()
        }
        0x8010_002E => "aucun lecteur de carte disponible".to_owned(),
        0x8010_000C => "aucune carte dans le lecteur".to_owned(),
        0x8010_0069 | 0x8010_0068 => "la carte a été retirée pendant la lecture".to_owned(),
        0x8010_000B => "la carte est déjà utilisée par un autre logiciel".to_owned(),
        0x8010_0017 => "le lecteur ne répond plus".to_owned(),
        0x8010_000A => "le lecteur n'a pas répondu à temps".to_owned(),
        other => format!("erreur PC/SC 0x{other:08X}"),
    }
}

/// A connection to the card service. Released when it goes out of scope.
pub struct Context {
    lib: &'static Lib,
    raw: Handle,
}

impl Context {
    pub fn open() -> Result<Context, String> {
        let lib = lib()?;
        let mut raw: Handle = 0;
        // SAFETY: the two reserved arguments are null as the API
        // requires, and `raw` is written only on success.
        let rc =
            unsafe { (lib.establish)(SCOPE_USER, std::ptr::null(), std::ptr::null(), &mut raw) };
        if rc != SUCCESS {
            return Err(message(rc));
        }
        Ok(Context { lib, raw })
    }

    /// Every reader the service knows about.
    ///
    /// Asked twice, as the API wants: once with no buffer, to be told the
    /// length, and once with a buffer that size. The answer is a run of
    /// NUL-terminated names closed by an empty one.
    pub fn readers(&self) -> Result<Vec<String>, String> {
        let mut len: Dword = 0;
        // SAFETY: a null buffer with a length out-parameter is the
        // documented way to ask for the size.
        let rc = unsafe {
            (self.lib.list_readers)(self.raw, std::ptr::null(), std::ptr::null_mut(), &mut len)
        };
        // An empty list is not an error worth a message of its own: the
        // caller says « aucun lecteur » better than the code does.
        if code_of(rc) == 0x8010_002E {
            return Ok(Vec::new());
        }
        if rc != SUCCESS {
            return Err(message(rc));
        }
        let mut buf = vec![0u8; len as usize];
        // SAFETY: the buffer is exactly the length the call just asked
        // for, and `len` is updated to what was written.
        let rc = unsafe {
            (self.lib.list_readers)(
                self.raw,
                std::ptr::null(),
                buf.as_mut_ptr() as *mut c_char,
                &mut len,
            )
        };
        if rc != SUCCESS {
            return Err(message(rc));
        }
        buf.truncate(len as usize);
        Ok(buf
            .split(|b| *b == 0)
            .filter(|name| !name.is_empty())
            .map(|name| String::from_utf8_lossy(name).into_owned())
            .collect())
    }

    /// Connect to the card sitting in one named reader.
    pub fn connect(&self, reader: &str) -> Result<Card<'_>, String> {
        let name = CString::new(reader).map_err(|_| "nom de lecteur illisible".to_owned())?;
        let mut raw: Handle = 0;
        let mut protocol: Dword = 0;
        // SAFETY: the name is a NUL-terminated C string that outlives the
        // call; the two handles are written only on success.
        let rc = unsafe {
            (self.lib.connect)(
                self.raw,
                name.as_ptr(),
                SHARE_SHARED,
                PROTOCOL_ANY,
                &mut raw,
                &mut protocol,
            )
        };
        if rc != SUCCESS {
            return Err(message(rc));
        }
        Ok(Card {
            lib: self.lib,
            raw,
            protocol,
            _ctx: std::marker::PhantomData,
        })
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // SAFETY: the handle came from a successful establish and is
        // released once.
        unsafe { (self.lib.release)(self.raw) };
    }
}

/// A card, connected. Disconnected — leaving it powered, as it was
/// found — when it goes out of scope.
pub struct Card<'a> {
    lib: &'static Lib,
    raw: Handle,
    protocol: Dword,
    _ctx: std::marker::PhantomData<&'a Context>,
}

impl Card<'_> {
    /// The card's ATR: the bytes it announces itself with. It says which
    /// card is in the reader, and it is the first thing to look at when
    /// nothing else works.
    pub fn atr(&self) -> Option<Vec<u8>> {
        let mut name = [0u8; 256];
        let mut name_len: Dword = name.len() as Dword;
        let mut state: Dword = 0;
        let mut protocol: Dword = 0;
        let mut atr = [0u8; 36];
        let mut atr_len: Dword = atr.len() as Dword;
        // SAFETY: every buffer is passed with its own length, and each
        // length is updated to what was written.
        let rc = unsafe {
            (self.lib.status)(
                self.raw,
                name.as_mut_ptr() as *mut c_char,
                &mut name_len,
                &mut state,
                &mut protocol,
                atr.as_mut_ptr(),
                &mut atr_len,
            )
        };
        let len = (atr_len as usize).min(atr.len());
        (rc == SUCCESS && len > 0).then(|| atr[..len].to_vec())
    }

    /// Send one APDU and return what the card answered, status word
    /// included.
    pub fn transmit(&self, command: &[u8]) -> Result<Vec<u8>, String> {
        let send = IoRequest {
            protocol: self.protocol,
            pci_length: std::mem::size_of::<IoRequest>() as Dword,
        };
        // Room for an extended-length answer: a command that answers with
        // more than the buffer holds fails outright rather than
        // truncating, and half a file read as a whole one is precisely
        // the kind of quiet wrong answer this module must not give.
        let mut answer = vec![0u8; 65_544];
        let mut len: Dword = answer.len() as Dword;
        // SAFETY: the command is read for its stated length, the answer
        // buffer is written for at most its own, and the receive header
        // is null — which the API allows.
        let rc = unsafe {
            (self.lib.transmit)(
                self.raw,
                &send,
                command.as_ptr(),
                command.len() as Dword,
                std::ptr::null_mut(),
                answer.as_mut_ptr(),
                &mut len,
            )
        };
        if rc != SUCCESS {
            return Err(message(rc));
        }
        answer.truncate((len as usize).min(answer.len()));
        Ok(answer)
    }
}

impl Drop for Card<'_> {
    fn drop(&mut self) {
        // Leave the card as it was found: another application on the same
        // post may be talking to it.
        //
        // SAFETY: the handle came from a successful connect and is
        // disconnected once.
        unsafe { (self.lib.disconnect)(self.raw, LEAVE_CARD) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_return_code_reads_the_same_on_a_32_and_a_64_bit_long() {
        // 0x8010001D is positive where LONG is 64 bits wide and negative
        // where it is 32; both must name the same failure.
        assert_eq!(code_of(0x8010_001D), 0x8010_001D);
        assert_eq!(code_of(-2_146_435_043), 0x8010_001D);
        assert_eq!(message(0x8010_001D), message(-2_146_435_043));
    }

    #[test]
    fn the_failures_of_a_counter_are_named_in_french() {
        assert!(message(0x8010_000C).contains("aucune carte"));
        assert!(message(0x8010_000B).contains("autre logiciel"));
        assert!(message(0x8010_0069).contains("retirée"));
        // Anything else still says enough to be reported.
        assert_eq!(message(0x8010_1234), "erreur PC/SC 0x80101234");
    }

    #[test]
    fn the_protocol_header_is_the_size_the_card_expects() {
        // Two 32-bit words on macOS, two 64-bit ones elsewhere: the
        // header carries its own length, and a wrong one is refused by
        // the library rather than read as data.
        assert_eq!(
            std::mem::size_of::<IoRequest>(),
            2 * std::mem::size_of::<Dword>()
        );
    }

    #[test]
    fn a_post_without_middleware_says_so_rather_than_failing_to_start() {
        // Whatever this machine has, asking must return, and must never
        // panic: that is the whole point of opening the library by name.
        match Context::open() {
            Ok(ctx) => {
                let _ = ctx.readers();
            }
            Err(e) => assert!(!e.is_empty()),
        }
    }
}
