// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Temporary files and directories

use io::{fs, IoResult};
use io;
use libc;
use ops::Drop;
use option::{Option, None, Some};
use os;
use path::{Path, GenericPath};
use result::{Ok, Err};
use sync::atomic;

/// A wrapper for a path to temporary directory implementing automatic
/// scope-based deletion.
pub struct TempDir {
    path: Option<Path>,
    disarmed: bool
}

impl TempDir {
    /// Attempts to make a temporary directory inside of `tmpdir` whose name
    /// will have the suffix `suffix`. The directory will be automatically
    /// deleted once the returned wrapper is destroyed.
    ///
    /// If no directory can be created, `Err` is returned.
    pub fn new_in(tmpdir: &Path, suffix: &str) -> IoResult<TempDir> {
        if !tmpdir.is_absolute() {
            return TempDir::new_in(&os::make_absolute(tmpdir), suffix);
        }

        static mut CNT: atomic::AtomicUint = atomic::INIT_ATOMIC_UINT;

        let mut attempts = 0u;
        loop {
            let filename =
                format!("rs-{}-{}-{}",
                        unsafe { libc::getpid() },
                        unsafe { CNT.fetch_add(1, atomic::SeqCst) },
                        suffix);
            let p = tmpdir.join(filename);
            match fs::mkdir(&p, io::USER_RWX) {
                Err(error) => {
                    if attempts >= 1000 {
                        return Err(error)
                    }
                    attempts += 1;
                }
                Ok(()) => return Ok(TempDir { path: Some(p), disarmed: false })
            }
        }
    }

    /// Attempts to make a temporary directory inside of `os::tmpdir()` whose
    /// name will have the suffix `suffix`. The directory will be automatically
    /// deleted once the returned wrapper is destroyed.
    ///
    /// If no directory can be created, `Err` is returned.
    pub fn new(suffix: &str) -> IoResult<TempDir> {
        TempDir::new_in(&os::tmpdir(), suffix)
    }

    /// Unwrap the wrapped `std::path::Path` from the `TempDir` wrapper.
    /// This discards the wrapper so that the automatic deletion of the
    /// temporary directory is prevented.
    pub fn unwrap(self) -> Path {
        let mut tmpdir = self;
        tmpdir.path.take().unwrap()
    }

    /// Access the wrapped `std::path::Path` to the temporary directory.
    pub fn path<'a>(&'a self) -> &'a Path {
        self.path.as_ref().unwrap()
    }

    /// Close and remove the temporary directory
    ///
    /// Although `TempDir` removes the directory on drop, in the destructor
    /// any errors are ignored. To detect errors cleaning up the temporary
    /// directory, call `close` instead.
    pub fn close(mut self) -> IoResult<()> {
        self.cleanup_dir()
    }

    fn cleanup_dir(&mut self) -> IoResult<()> {
        assert!(!self.disarmed);
        self.disarmed = true;
        match self.path {
            Some(ref p) => {
                fs::rmdir_recursive(p)
            }
            None => Ok(())
        }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        if !self.disarmed {
            let _ = self.cleanup_dir();
        }
    }
}

// the tests for this module need to change the path using change_dir,
// and this doesn't play nicely with other tests so these unit tests are located
// in src/test/run-pass/tempfile.rs
