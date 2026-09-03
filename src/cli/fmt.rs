use std::time::Duration;

pub fn fmt_int(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    let mut first = s.len() % 3;
    if first == 0 {
        first = 3;
    }
    out.push_str(&s[..first]);
    let mut i = first;
    while i < s.len() {
        out.push(',');
        out.push_str(&s[i..i + 3]);
        i += 3;
    }
    out
}

pub fn fmt_rate(mut r: f64) -> String {
    if !r.is_finite() || r < 0.0 {
        r = 0.0;
    }
    const U: [&str; 5] = ["", "k", "M", "G", "T"];
    let mut u = 0;
    while r >= 1000.0 && u < U.len() - 1 {
        r /= 1000.0;
        u += 1;
    }
    if r >= 100.0 {
        format!("{:.0}{}", r, U[u])
    } else if r >= 10.0 {
        format!("{:.1}{}", r, U[u])
    } else {
        format!("{:.2}{}", r, U[u])
    }
}

pub fn fmt_duration(d: Duration) -> String {
    let mut s = d.as_secs();
    let h = s / 3600;
    s %= 3600;
    let m = s / 60;
    let sec = s % 60;
    if h > 0 {
        format!("{:02}:{:02}:{:02}", h, m, sec)
    } else {
        format!("{:02}:{:02}", m, sec)
    }
}
