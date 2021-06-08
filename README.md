# shufflebuf

A simple Rust no_std buffer implementation using slices.
This is probably less optimal than a ring buffer for
most applications, but I find it easy to use and debug
communications drivers using something like this. 

## Status

- [x] Works
- [x] Tests
- [x] Generic over buffer length (currently fixed length buffer)
- [ ] Theoretically Awesome
