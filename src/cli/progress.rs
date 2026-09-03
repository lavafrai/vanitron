use crate::cli::fmt::{fmt_duration, fmt_int, fmt_rate};
use std::collections::VecDeque;
use std::io::Write;

pub struct ProgressBar {
    start_time: std::time::Instant,
    last_tick: std::time::Instant,
    last_tries: u64,
    spinner_idx: usize,
    smoothed_cur: Option<f64>,
    alpha: f64,
    samples: VecDeque<(std::time::Instant, u64)>,
    window: std::time::Duration,
}

impl ProgressBar {
    pub fn new(start_time: std::time::Instant) -> Self {
        Self {
            start_time,
            last_tick: std::time::Instant::now(),
            last_tries: 0,
            spinner_idx: 0,
            smoothed_cur: None,
            alpha: 0.35,
            samples: VecDeque::with_capacity(600),
            window: std::time::Duration::from_secs(60),
        }
    }

    pub fn update(&mut self, tries: u64) {
        const PROGRESS_CHARS: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

        let ch = PROGRESS_CHARS[self.spinner_idx % PROGRESS_CHARS.len()];
        self.spinner_idx = self.spinner_idx.wrapping_add(1);

        let now = std::time::Instant::now();
        let elapsed_total = now.duration_since(self.start_time);

        self.samples.push_back((now, tries));
        let cutoff = now - self.window;
        while let Some((t, _)) = self.samples.front().copied() {
            if t < cutoff {
                self.samples.pop_front();
            } else {
                break;
            }
        }

        let dt = self.last_tick.elapsed().as_secs_f64().max(1e-9);
        let dtries = tries.saturating_sub(self.last_tries) as f64;
        let rate_last = dtries / dt;

        let rate_avg = if let Some((old_t, old_tries)) = self.samples.front().copied() {
            let dtw = (now - old_t).as_secs_f64();
            if dtw > 0.5 {
                let dtr = tries.saturating_sub(old_tries) as f64;
                dtr / dtw
            } else {
                rate_last
            }
        } else {
            rate_last
        };

        let short_cutoff = now - std::time::Duration::from_secs(3);
        let (short_t, short_tries) = self
            .samples
            .iter()
            .copied()
            .find(|(time, _)| *time >= short_cutoff)
            .unwrap_or((self.start_time, 0));
        let short_elapsed = now.duration_since(short_t).as_secs_f64();
        let short_rate = if short_elapsed > 0.0 {
            tries.saturating_sub(short_tries) as f64 / short_elapsed
        } else {
            rate_last
        };
        let smoothed_cur = match self.smoothed_cur {
            Some(prev) => prev + self.alpha * (short_rate - prev),
            None => short_rate,
        };
        self.smoothed_cur = Some(smoothed_cur);

        let trend = if rate_avg.is_finite() && rate_avg > 0.0 {
            let rel = (smoothed_cur - rate_avg) / rate_avg;
            if rel > 0.05 {
                '↑'
            } else if rel > 0.01 {
                '↗'
            } else if rel < -0.05 {
                '↓'
            } else if rel < -0.01 {
                '↘'
            } else {
                '→'
            }
        } else if smoothed_cur.is_finite() && smoothed_cur > 0.0 {
            '↑'
        } else {
            ' '
        };

        let line = format!(
            "{} Tries: {} | Avg: {}/s {} | Cur: {}/s | Elapsed: {}",
            ch,
            fmt_int(tries),
            fmt_rate(rate_avg),
            trend,
            fmt_rate(smoothed_cur),
            fmt_duration(elapsed_total),
        );

        print!("\r{}\x1B[K", line);
        let _ = std::io::stdout().flush();

        self.last_tick = now;
        self.last_tries = tries;
    }

    pub fn finish(&self) {
        let now = std::time::Instant::now();
        let elapsed_total = now.duration_since(self.start_time);

        let rate_avg = if let Some((old_t, old_tries)) = self.samples.front().copied() {
            let dtw = (now - old_t).as_secs_f64();
            if dtw > 0.0 {
                let dtr = self.last_tries.saturating_sub(old_tries) as f64;
                dtr / dtw
            } else {
                0.0
            }
        } else {
            0.0
        };

        let line = format!(
            "⣿ Tries: {} | Average rate: {}/s | Elapsed: {}",
            fmt_int(self.last_tries),
            fmt_rate(rate_avg),
            fmt_duration(elapsed_total),
        );

        print!("\r{}\x1B[K", line);
        println!();
    }
}
