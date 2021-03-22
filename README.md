# Nippy &emsp; [![Build Status]][travis] [![Latest Version]][crates.io] [![Docs]][docs.rs]

[Build Status]: https://travis-ci.com/apibillme/nippy.svg?branch=master
[travis]: https://travis-ci.com/apibillme/nippy
[Latest Version]: https://img.shields.io/crates/v/nippy.svg
[crates.io]: https://crates.io/crates/nippy
[Docs]: https://docs.rs/nippy/badge.svg
[docs.rs]: https://docs.rs/nippy

### Purpose

The purpose of this library is to be your async ntp utility.

This is a fork of the crate `ntp` that adds support for `async-std` and `Rust 2018`.

### Use

``` rust

nippy::get_unix_ntp_time().await.unwrap();

```

This will return an i64 that is the unix ntp timestamp from `pool ntp server`.
