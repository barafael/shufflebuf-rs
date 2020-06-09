/*
Copyright (c) 2020 Todd Stellanova
LICENSE: BSD3 (see LICENSE file)
*/

#![cfg_attr(not(test), no_std)]

pub const BUF_LEN: usize = 256;
use core::sync::atomic::{AtomicUsize, Ordering::SeqCst};

pub struct ShuffleBuf {
    /// The actual buffer
    buf: [u8; BUF_LEN],
    /// The index at which the next byte should be read from the buffer
    read_idx: AtomicUsize,
    /// The index at which the next byte should be written to the buffer
    write_idx: AtomicUsize,
}

/// Simple buffer implementation using slices
impl ShuffleBuf {
    pub fn default() -> Self {
        Self {
            buf: [0; BUF_LEN],
            read_idx: AtomicUsize::new(0),
            write_idx: AtomicUsize::new(0),
        }
    }

    /// Read one byte from the buffer
    /// Returns the number of bytes returned (0 or 1)
    pub fn read_one(&mut self) -> (usize, u8) {
        let read_idx = self.read_idx.load(SeqCst);
        let write_idx = self.write_idx.load(SeqCst);
        if write_idx > read_idx {
            let val = self.buf[read_idx];
            self.read_idx.fetch_add(1, SeqCst);
            if self.read_idx.load(SeqCst) > 4 {
                self.shuffle_up();
            }
            return (1 as usize, val);
        }
        (0, 0)
    }

    /// Pull some data out of the buffer
    /// Returns the number of bytes returned (`out_buf.len()` max)
    pub fn read_many(&mut self, out_buf: &mut [u8]) -> usize {
        let mut read_count = 0;
        let write_idx = self.write_idx.load(SeqCst);
        let read_idx = self.read_idx.load(SeqCst);
        let avail = write_idx - read_idx;
        if avail > 0 {
            let desired = out_buf.len();
            if desired > avail {
                read_count = avail;
            } else {
                read_count = desired;
            }
            out_buf[..read_count]
                .copy_from_slice(&self.buf[read_idx..read_idx + read_count]);
            //update pointers
            self.read_idx.fetch_add(read_count, SeqCst);
            self.shuffle_up();
        }
        read_count
    }

    /// How much data is available to read?
    pub fn available(&self) -> usize {
        self.write_idx.load(SeqCst) - self.read_idx.load(SeqCst)
    }

    /// How much space is vacant in the buffer?
    pub fn vacant(&self) -> usize {
        self.buf.len() - self.write_idx.load(SeqCst)
    }

    /// Move remaining bytes to the start of the buffer
    fn shuffle_up(&mut self) {
        let read_idx = self.read_idx.load(SeqCst);
        if read_idx > 0 {
            let write_idx = self.write_idx.load(SeqCst);
            let avail = write_idx - read_idx;
            if avail > 0 {
                self.buf.copy_within(read_idx..write_idx, 0);
                self.read_idx.store(0, SeqCst);
                self.write_idx.store(avail, SeqCst);
            }
        }
    }

    /// Push one byte into the buffer
    pub fn push_one(&mut self, data: u8) -> usize {
        let write_idx = self.write_idx.load(SeqCst);
        if self.buf.len() > write_idx {
            self.buf[write_idx] = data;
            self.write_idx.fetch_add(1, SeqCst);
            1
        } else {
            0
        }
    }

    /// Copy some data into the buffer
    pub fn push_many(&mut self, data: &[u8]) -> usize {
        let mut copy_count = 0;
        let vacant = self.vacant();
        if vacant > 0 {
            let desired = data.len();
            if desired < vacant {
                copy_count = desired;
            } else {
                copy_count = vacant;
            }
            let write_idx = self.write_idx.load(SeqCst);
            self.buf[write_idx..write_idx + copy_count]
                .copy_from_slice(data[..copy_count].as_ref());
            self.write_idx.fetch_add(copy_count, SeqCst);
        }
        copy_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use core::time::Duration;
    use std::thread;
    use lazy_static::lazy_static;
    use std::sync::{Mutex};


    #[test]
    fn test_basics() {
        let buf_a: [u8; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

        let mut shuffler = ShuffleBuf::default();
        let push_count = shuffler.push_many(&buf_a);
        assert_eq!(push_count, buf_a.len());

        let mut buf_b = [0u8; 25];
        let read_count = shuffler.read_many(&mut buf_b);
        assert_eq!(read_count, 10); //same as buf_a
                                    // no more bytes left
        let read_count = shuffler.read_many(&mut buf_b);
        assert_eq!(read_count, 0);
    }

    #[test]
    fn test_overrun() {
        let mut buf_a: [u8; 512] = [8; 512];
        buf_a[55] = 127;

        let mut shuffler = ShuffleBuf::default();
        let push_count = shuffler.push_many(&buf_a);
        assert_eq!(push_count, BUF_LEN);
        assert_eq!(shuffler.available(), BUF_LEN);
        assert_eq!(shuffler.vacant(), 0);

        buf_a[55] = 0;
        let read_count = shuffler.read_many(buf_a[..60].as_mut());
        assert_eq!(read_count, 60);
        assert_eq!(buf_a[55], 127); //original value

        assert_eq!(shuffler.available(), BUF_LEN - 60);
    }

    #[test]
    fn test_write_read_write_read() {
        let buf_a: [u8; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut shuffler = ShuffleBuf::default();
        shuffler.push_many(&buf_a);
        shuffler.push_many(&buf_a);
        shuffler.push_many(&buf_a);
        assert_eq!(shuffler.available(), buf_a.len() * 3);

        let mut buf_b: [u8; 15] = [0; 15];
        let read_count = shuffler.read_many(buf_b.as_mut());
        assert_eq!(read_count, 15);
        assert_eq!(shuffler.available(), 15);
        assert_eq!(buf_b[14], 4);

        shuffler.push_many(&buf_a);
        assert_eq!(shuffler.available(), 25);

        let mut buf_c: [u8; 40] = [0; 40];
        let read_count = shuffler.read_many(buf_c.as_mut());
        assert_eq!(read_count, 25);
        assert_eq!(buf_c[24], 9);
    }

    #[test]
    fn test_single_pushes_multi_read() {
        let mut shuffler = ShuffleBuf::default();

        for i in 0..100 {
            shuffler.push_one(i as u8);
        }
        assert_eq!(shuffler.available(), 100);
        let mut read_bytes = [0u8; 50];
        shuffler.read_many(&mut read_bytes);
        assert_eq!(shuffler.vacant(), BUF_LEN - 50);
        assert_eq!(read_bytes[49], 49);
    }

    #[test]
    fn multithread_read() {
        lazy_static!{
            static ref TOTAL_READ_COUNT:AtomicUsize = AtomicUsize::new(0);
            static ref INNER_READ_COUNT:AtomicUsize = AtomicUsize::new(0);
            static ref SHUFFALO: Mutex<ShuffleBuf> = Mutex::new(ShuffleBuf::default());
        };

        for i in 0..100 {
            SHUFFALO.lock().unwrap().push_one(i as u8);
        }
        assert_eq!(SHUFFALO.lock().unwrap().available(), 100);

        let inner_thread = thread::spawn(|| {
            for _ in 0..100 {
                let (nread,_b) =  SHUFFALO.lock().unwrap().read_one();
                TOTAL_READ_COUNT.fetch_add(nread, SeqCst);
                INNER_READ_COUNT.fetch_add(nread, SeqCst);
                thread::yield_now();
            }
        });

        let mut outer_thread_read_count = 0;
        for _ in 0..100 {
            let (nread,_b) = SHUFFALO.lock().unwrap().read_one();
            TOTAL_READ_COUNT.fetch_add(nread, SeqCst);
            outer_thread_read_count += nread;
            thread::yield_now();
        }
        println!("outer_thread_read_count: {}", outer_thread_read_count);
        inner_thread.join().unwrap();
        println!("inner_thread_read_count: {}", INNER_READ_COUNT.load(SeqCst));

        assert_eq!(outer_thread_read_count + INNER_READ_COUNT.load(SeqCst), 100);
        assert_eq!(TOTAL_READ_COUNT.load(SeqCst), 100);

    }
}
