/*
Copyright (c) 2020 Todd Stellanova
LICENSE: BSD3 (see LICENSE file)
*/

#![no_std]

pub const BUF_LEN: usize = 256;

pub struct ShuffleBuf {
    buf: [u8; BUF_LEN],
    read_idx: usize,
    write_idx: usize,
}


impl ShuffleBuf {
    pub fn default() -> Self {
        Self {
            buf: [0; BUF_LEN],
            read_idx: 0,
            write_idx: 0,
        }
    }

    pub fn read_one(&mut self) -> (usize, u8) {
        if self.write_idx > self.read_idx {
            let val = self.buf[self.read_idx];
            self.read_idx += 1;
            self.shuffle_up();
            return (1 as usize, val);
        }
        (0,0)
    }

    /// Pull some data out of the buffer
    pub fn read_many(&mut self, out_buf: &mut [u8]) -> usize {
        let mut read_count = 0;
        if self.write_idx > 0 {
            let desired = out_buf.len();
            let avail = self.write_idx - self.read_idx;
            if desired > avail {
                read_count = avail;
            }
            else {
                read_count = desired;
            }
            out_buf[..read_count].copy_from_slice(&self.buf[self.read_idx..self.read_idx+read_count]);
            //update pointers
            self.read_idx += read_count;
            self.shuffle_up();
        }
        read_count
    }

    /// How much data is available to read?
    pub fn available(&self) -> usize {
        self.write_idx - self.read_idx
    }

    /// how much space is vacant in the buffer?
    pub fn vacant(&self) -> usize {
        self.buf.len() - self.write_idx
    }

    /// Move remaining bytes to the start of the buffer
    fn shuffle_up(&mut self) {
        if self.read_idx > 0 {
            let avail = self.write_idx - self.read_idx;
            if avail > 0 {
                self.buf.copy_within(self.read_idx..self.write_idx, 0);
            }
            self.read_idx = 0;
            self.write_idx = avail;
        }
    }

    pub fn push_one(&mut self, data: u8) -> usize {
        if self.buf.len() > self.write_idx {
            self.buf[self.write_idx] = data;
            self.write_idx += 1;
            1
        }
        else {
            0
        }
    }

    /// Copy some data into the buffer
    pub fn push_many(&mut self, data: &[u8]) -> usize {
        let mut copy_count = 0;
        let vacant = self.buf.len() - self.write_idx;
        if vacant > 0 {
            let desired = data.len();
            if desired < vacant {
                copy_count = desired;
            }
            else {
                copy_count = vacant;
            }
            self.buf[self.write_idx..self.write_idx+ copy_count].copy_from_slice(data[..copy_count].as_ref());
            self.write_idx += copy_count;
        }
        copy_count
    }

}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(shuffler.available(), buf_a.len()*3);

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
        assert_eq!(shuffler.available(),100);
        let mut read_bytes = [0u8; 50];
        shuffler.read_many(&mut read_bytes);
        assert_eq!(shuffler.vacant(), BUF_LEN - 50);
        assert_eq!(read_bytes[49], 49);
    }
}
