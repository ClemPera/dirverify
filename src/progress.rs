use std::fmt;
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::report;

/// How often the ticker redraws the live line.
const TICK: Duration = Duration::from_millis(150);
/// Interval between plain progress lines when stderr is not a terminal.
const PLAIN_INTERVAL: Duration = Duration::from_secs(30);
/// Current file name is clipped to this many characters.
const NAME_WIDTH: usize = 48;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum When {
    Auto,
    Always,
    Never,
}

/// Progress goes to stderr; stdout keeps only the report.
pub struct Progress {
    inner: Option<Inner>,
}

struct Inner {
    shared: Arc<Shared>,
    stop: Arc<AtomicBool>,
    ticker: Mutex<Option<thread::JoinHandle<()>>>,
}

struct Shared {
    state: Mutex<State>,
    bytes: AtomicU64,
    files: AtomicU64,
    inline: bool,
}

struct State {
    label: String,
    total_bytes: u64,
    current: String,
    started: Instant,
    drawn: bool,
    last_plain: Instant,
}

impl Progress {
    pub fn new(when: When) -> Self {
        let inline = io::stderr().is_terminal();
        let enabled = match when {
            When::Never => false,
            When::Always => true,
            When::Auto => inline,
        };
        if !enabled {
            return Self { inner: None };
        }

        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                label: String::new(),
                total_bytes: 0,
                current: String::new(),
                started: Instant::now(),
                drawn: false,
                last_plain: Instant::now(),
            }),
            bytes: AtomicU64::new(0),
            files: AtomicU64::new(0),
            inline,
        });
        let stop = Arc::new(AtomicBool::new(false));
        let ticker = {
            let shared = Arc::clone(&shared);
            let stop = Arc::clone(&stop);
            thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    thread::sleep(TICK);
                    shared.render();
                }
            })
        };

        Self {
            inner: Some(Inner {
                shared,
                stop,
                ticker: Mutex::new(Some(ticker)),
            }),
        }
    }

    /// Stops the ticker and clears the live line. Call before printing the final report.
    pub fn finish(&self) {
        let Some(inner) = &self.inner else { return };
        inner.stop.store(true, Ordering::Relaxed);
        let ticker = inner.ticker.lock().unwrap().take();
        if let Some(ticker) = ticker {
            let _ = ticker.join();
        }
        inner.end_step();
    }

    /// Ends the previous step and starts a new one.
    pub fn step(&self, label: &str) {
        let Some(inner) = &self.inner else { return };
        inner.end_step();

        let mut state = inner.shared.state.lock().unwrap();
        state.label = label.to_string();
        state.total_bytes = 0;
        state.current.clear();
        state.started = Instant::now();
        state.last_plain = Instant::now();
        inner.shared.bytes.store(0, Ordering::Relaxed);
        inner.shared.files.store(0, Ordering::Relaxed);
    }

    /// Total work for the current step, enabling percentage and ETA.
    pub fn total(&self, bytes: u64) {
        if let Some(inner) = &self.inner {
            inner.shared.state.lock().unwrap().total_bytes = bytes;
        }
    }

    pub fn advance(&self, bytes: u64) {
        if let Some(inner) = &self.inner {
            inner.shared.bytes.fetch_add(bytes, Ordering::Relaxed);
        }
    }

    pub fn saw_file(&self, path: &Path) {
        let Some(inner) = &self.inner else { return };
        inner.shared.files.fetch_add(1, Ordering::Relaxed);
        inner.shared.state.lock().unwrap().current = tail(&path.display().to_string(), NAME_WIDTH);
    }

    /// Report line, keeping the live progress line intact.
    pub fn println(&self, args: fmt::Arguments<'_>) {
        match &self.inner {
            None => println!("{args}"),
            Some(inner) => {
                inner.shared.clear();
                println!("{args}");
            }
        }
    }

    pub fn eprintln(&self, args: fmt::Arguments<'_>) {
        match &self.inner {
            None => eprintln!("{args}"),
            Some(inner) => {
                inner.shared.clear();
                eprintln!("{args}");
            }
        }
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        self.finish();
    }
}

impl Inner {
    fn end_step(&self) {
        let mut state = self.shared.state.lock().unwrap();
        if state.label.is_empty() {
            return;
        }
        if self.shared.inline {
            if state.drawn {
                let mut err = io::stderr().lock();
                let _ = write!(err, "\r\x1b[K");
                let _ = err.flush();
            }
            state.drawn = false;
        } else {
            let line = state.line(
                self.shared.bytes.load(Ordering::Relaxed),
                self.shared.files.load(Ordering::Relaxed),
            );
            let mut err = io::stderr().lock();
            let _ = writeln!(err, "{line}");
            let _ = err.flush();
        }
        state.label.clear();
    }
}

impl Shared {
    fn render(&self) {
        let mut state = self.state.lock().unwrap();
        if state.label.is_empty() {
            return;
        }
        if !self.inline && state.last_plain.elapsed() < PLAIN_INTERVAL {
            return;
        }

        let line = state.line(
            self.bytes.load(Ordering::Relaxed),
            self.files.load(Ordering::Relaxed),
        );
        let mut err = io::stderr().lock();
        if self.inline {
            let _ = write!(err, "\r{line}\x1b[K");
        } else {
            let _ = writeln!(err, "{line}");
            state.last_plain = Instant::now();
        }
        let _ = err.flush();
        state.drawn = true;
    }

    fn clear(&self) {
        let mut state = self.state.lock().unwrap();
        if !state.drawn {
            return;
        }
        if self.inline {
            let mut err = io::stderr().lock();
            let _ = write!(err, "\r\x1b[K");
            let _ = err.flush();
        }
        state.drawn = false;
    }
}

impl State {
    fn line(&self, bytes: u64, files: u64) -> String {
        let elapsed = self.started.elapsed().as_secs_f64().max(0.001);

        if self.total_bytes == 0 {
            return format!(
                "{}: {} files, {}",
                self.label,
                thousands(files),
                report::size(bytes)
            );
        }

        let percent = (bytes as f64 / self.total_bytes as f64 * 100.0).min(100.0);
        let rate = (bytes as f64 / elapsed) as u64;
        let eta = self
            .total_bytes
            .saturating_sub(bytes)
            .checked_div(rate)
            .unwrap_or(0);
        let mut line = format!(
            "{}: {:>5.1}%  {} / {}  {}/s  eta {}",
            self.label,
            percent,
            report::size(bytes),
            report::size(self.total_bytes),
            report::size(rate),
            duration(eta)
        );
        if !self.current.is_empty() {
            line.push_str("  ");
            line.push_str(&self.current);
        }
        line
    }
}

fn thousands(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

fn duration(seconds: u64) -> String {
    let (hours, minutes, seconds) = (seconds / 3600, seconds % 3600 / 60, seconds % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

/// Keep the informative end of a long path.
fn tail(text: &str, width: usize) -> String {
    let count = text.chars().count();
    if count <= width {
        return text.to_string();
    }
    let skip = count - width + 1;
    format!("…{}", text.chars().skip(skip).collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousands_separates_groups() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1000), "1,000");
        assert_eq!(thousands(12483), "12,483");
        assert_eq!(thousands(1234567), "1,234,567");
    }

    #[test]
    fn durations_switch_to_hours_when_needed() {
        assert_eq!(duration(9), "0:09");
        assert_eq!(duration(75), "1:15");
        assert_eq!(duration(3725), "1:02:05");
    }

    #[test]
    fn tail_clips_the_start_of_long_paths() {
        assert_eq!(tail("short.txt", 20), "short.txt");
        assert_eq!(tail("abcdefghij", 5), "…ghij");
    }
}
