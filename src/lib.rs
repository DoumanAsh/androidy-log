//! Minimal wrapper over Android logging facilities.
//!
//! ## Features:
//!
//! - `std` - Enables `std::io::Write` implementation.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use androidy_log::{LogPriority, Writer};
//!
//! use core::fmt::Write;
//!
//! let mut writer = Writer::new("MyTag", LogPriority::INFO);
//! let _ = write!(writer, "Hellow World!");
//! drop(writer); //or writer.flush();
//!
//! androidy_log::println!("Hello via macro!");
//! androidy_log::eprintln!("Error via macro!");
//! ```
//!

#![cfg_attr(not(test), no_std)]
#![warn(missing_docs)]

#[cfg(feature = "std")]
extern crate std;

use core::{cmp, mem, ptr, fmt};

///Priority of the log message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum LogPriority {
    ///For internal use only.
    UNKNOWN = 0,
    ///The default priority, for internal use only.
    DEFAULT = 1,
    ///Verbose logging.
    VERBOSE = 2,
    ///Debug logging.
    DEBUG = 3,
    ///Informational logging.
    INFO = 4,
    ///Warning logging.
    ///
    ///For use with recoverable failures.
    WARN = 5,
    ///Error logging.
    ///
    ///For use with unrecoverable failures.
    ERROR = 6,
    ///Fatal logging.
    ///
    ///For use when aborting.
    FATAL = 7,
    ///For internal use only.
    SILENT = 8,
}

const TAG_MAX_LEN: usize = 23;
//Re-check NDK sources, I think internally kernel limits to 4076, but
//it includes some overhead of logcat machinery, hence 4000
//Don't remember details
const BUFFER_CAPACITY: usize = 4000;
const DEFAULT_TAG: &str = "Rust";

#[cfg(not(test))]
#[link(name = "log")]
extern "C" {
    fn __android_log_write(prio: i32, tag: *const i8, text: *const i8) -> i32;
}

#[cfg(test)]
fn __android_log_write(_: i32, _: *const i8, _: *const i8) -> i32 {
    0
}

///Android log writer.
///
///By default every write is buffer unless buffer overflow happens.
///Buffered input is flushed on `Drop` or via manual call.
pub struct Writer {
    //Null character is not within limit
    tag: mem::MaybeUninit<[u8; TAG_MAX_LEN + 1]>,
    prio: LogPriority,
    //Null character is not within limit
    buffer: mem::MaybeUninit<[u8; BUFFER_CAPACITY + 1]>,
    len: usize,
}

impl Writer {
    #[inline(always)]
    ///Creates new instance using default tag `Rust`
    ///
    ///- `prio` - Logging priority.
    pub const fn new_default(prio: LogPriority) -> Self {
        let mut tag = [0u8; TAG_MAX_LEN + 1];

        tag[0] = DEFAULT_TAG.as_bytes()[0];
        tag[1] = DEFAULT_TAG.as_bytes()[1];
        tag[2] = DEFAULT_TAG.as_bytes()[2];
        tag[3] = DEFAULT_TAG.as_bytes()[3];
        unsafe {
            Self::from_raw_parts(mem::MaybeUninit::new(tag), prio)
        }
    }

    #[inline]
    ///Creates new instance using:
    ///
    ///- `tag` - Log message tag, truncated to first 23 characters.
    ///- `prio` - Logging priority
    pub fn new(tag: &str, prio: LogPriority) -> Self {
        let mut tag_buffer = mem::MaybeUninit::<[u8; TAG_MAX_LEN + 1]>::uninit();
        unsafe {
            ptr::copy_nonoverlapping(tag.as_ptr(), tag_buffer.as_mut_ptr() as *mut u8, cmp::min(tag.len(), TAG_MAX_LEN));
            (tag_buffer.as_mut_ptr() as *mut u8).add(TAG_MAX_LEN).write(0);
            Self::from_raw_parts(tag_buffer, prio)
        }
    }

    #[inline]
    ///Creates new instance with:
    ///
    ///- `tag` - Log message's tag as raw C string, that must be ending with 0. It is UB to pass anything else.
    ///- `prio` - Logging priority
    pub const unsafe fn from_raw_parts(tag: mem::MaybeUninit<[u8; TAG_MAX_LEN + 1]>, prio: LogPriority) -> Self {
        Self {
            tag,
            prio,
            buffer: mem::MaybeUninit::uninit(),
            len: 0,
        }
    }

    #[inline(always)]
    ///Returns content of written buffer.
    pub fn buffer(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(self.buffer.as_ptr() as *const u8, self.len)
        }
    }

    #[inline(always)]
    fn as_mut_ptr(&mut self) -> *mut u8 {
        self.buffer.as_mut_ptr() as _
    }

    #[inline(always)]
    ///Flushes internal buffer, if any data is available.
    ///
    ///Namely it dumps stored data in buffer via `__android_log_write`.
    ///And resets buffered length to 0.
    pub fn flush(&mut self) {
        if self.len > 0 {
            self.inner_flush();
        }
    }

    fn inner_flush(&mut self) {
        unsafe {
            (self.buffer.as_mut_ptr() as *mut u8).add(self.len).write(0);
            __android_log_write(self.prio as _, self.tag.as_ptr() as _, self.buffer.as_ptr() as *const _);
        }
        self.len = 0;
    }

    #[inline]
    fn copy_data<'a>(&mut self, text: &'a [u8]) -> &'a [u8] {
        let write_len = cmp::min(BUFFER_CAPACITY.saturating_sub(self.len), text.len());
        unsafe {
            ptr::copy_nonoverlapping(text.as_ptr(), self.as_mut_ptr().add(self.len), write_len);
        }
        self.len += write_len;
        &text[write_len..]
    }

    ///Writes supplied text to the buffer.
    ///
    ///On buffer overflow, data is logged via `__android_log_write`
    ///and buffer is filled with the rest of `data`
    pub fn write_data(&mut self, mut data: &[u8]) {
        loop {
            data = self.copy_data(data);

            if data.is_empty() {
                break;
            } else {
                self.flush();
            }
        }
    }
}

impl fmt::Write for Writer {
    #[inline]
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.write_data(text.as_bytes());

        Ok(())
    }
}

#[cfg(feature = "std")]
impl std::io::Write for Writer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.write_data(buf);
        Ok(buf.len())
    }

    #[inline(always)]
    fn flush(&mut self) -> std::io::Result<()> {
        self.flush();
        Ok(())
    }
}

impl Drop for Writer {
    #[inline]
    fn drop(&mut self) {
        self.flush();
    }
}

#[macro_export]
///`println` alternative to write message with INFO priority.
macro_rules! println {
    () => {{
        $crate::println!(" ");
    }};
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let mut writer = $crate::Writer::new_default($crate::LogPriority::INFO);
        let _ = write!(writer, $($arg)*);
        drop(writer);
    }}
}

#[macro_export]
///`eprintln` alternative to write message with ERROR priority.
macro_rules! eprintln {
    () => {{
        $crate::println!(" ");
    }};
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let mut writer = $crate::Writer::new_default($crate::LogPriority::ERROR);
        let _ = write!(writer, $($arg)*);
        drop(writer);
    }}
}

#[cfg(test)]
mod tests {
    use super::{LogPriority, Writer, TAG_MAX_LEN, DEFAULT_TAG};
    const TAG: &str = "Test";
    const TAG_OVERFLOW: &str = "123456789123456789123456789";

    #[test]
    fn should_truncate_tag() {
        let writer = Writer::new(TAG_OVERFLOW, LogPriority::WARN);
        assert!(TAG_OVERFLOW.len() > TAG_MAX_LEN);
        let tag = unsafe { core::slice::from_raw_parts(writer.tag.as_ptr() as *const u8, TAG_MAX_LEN) };
        assert_eq!(tag, TAG_OVERFLOW[..TAG_MAX_LEN].as_bytes());
    }

    #[test]
    fn should_normal_write() {
        let mut writer = Writer::new_default(LogPriority::WARN);

        let tag = unsafe { core::slice::from_raw_parts(writer.tag.as_ptr() as *const u8, DEFAULT_TAG.len()) };
        assert_eq!(tag, DEFAULT_TAG.as_bytes());
        assert_eq!(writer.prio, LogPriority::WARN);

        let data = TAG_OVERFLOW.as_bytes();

        writer.write_data(data);
        assert_eq!(writer.len, data.len());
        assert_eq!(writer.buffer(), data);

        writer.write_data(b" ");
        writer.write_data(data);
        let expected = format!("{} {}", TAG_OVERFLOW, TAG_OVERFLOW);
        assert_eq!(writer.len, expected.len());
        assert_eq!(writer.buffer(), expected.as_bytes());
    }

    #[test]
    fn should_handle_write_overflow() {
        let mut writer = Writer::new(TAG, LogPriority::WARN);
        let data = TAG_OVERFLOW.as_bytes();
        assert_eq!(unsafe { core::slice::from_raw_parts(writer.tag.as_ptr() as *const u8, TAG.len() + 1) }, &b"Test\0"[..]);

        //BUFFER_CAPACITY / TAG_OVERFLOW.len() = 148.xxx
        for idx in 1..=148 {
            writer.write_data(data);
            assert_eq!(writer.len, data.len() * idx);
        }

        writer.write_data(data);
        assert_eq!(writer.len, 23);
    }
}
