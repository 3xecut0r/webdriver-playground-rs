# webdriver-playground-rs

A small Rust utility that utomatically downloads, validates, and runs ChromeDriver, so you can start writing browser automation immediately without dealing with driver versions, paths, or manual setup.

This project is designed as a bootstrap layer for Selenium / WebDriver in Rust.

---

## Why this exists

If you have ever worked with Selenium, you know the pain:

- manually downloading ChromeDriver
- matching Chrome driver versions
- managing PATH and binaries
- starting and stopping the driver process
- random "driver not found" errors

This tool handles all of that for you:
- detects OS and CPU arch:
- downloads the last-known-good ChromeDriver
- unpacks it locally
- verifies that it actually runs
- starts ChromeDriver as a child process
- cleanly shuts it down on exit

---

## What it does

- Supports:
   - Linux (x86_64 / arm64)
   - macOS (Intel / Apple Silicon)
   - Windows (x86 / x64)
- No global instanllation required
- Stores the driver locally (`./bin/chromedriver`)
- Ensures the driver is executable and valid
- Ready to use with `thirtyfour` (Selenium for Rust)

---

## How it works (high level)

1. On startup:
   - checks for `chromedriver` in `./bin`

2. If missing or invalid:
   - downloads the official ChromeDriver
   - selects the correct platform
   - extracts and marks it executable

3. Starts ChromeDriver as a child process
4. Waits until the WebDriver `/status` endpoint is ready
5. You can immediately control browser

--- 

## Who this is for

- Rust developers
- Web scraping
- Browser automation
- End-to-end testing
- Anyone who wants WebDriver without the setup hassle

## TL;DR

> **Run your binary — you get a working WebDriver.**
>
> No downloads, no PATH setup, no pain.
