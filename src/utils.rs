//! Shared utility functions for nofs

/// SI unit constants for size
pub const KB: u64 = 1000;
pub const MB: u64 = KB * 1000;
pub const GB: u64 = MB * 1000;
pub const TB: u64 = GB * 1000;
pub const PB: u64 = TB * 1000;

/// Format size in human-readable format (SI units)
#[allow(
    clippy::cast_precision_loss,
    clippy::as_conversions,
    clippy::float_arithmetic
)]
pub fn format_size(size: u64) -> String {
    if size >= PB {
        format!("{:.1} PB", size as f64 / PB as f64)
    } else if size >= TB {
        format!("{:.1} TB", size as f64 / TB as f64)
    } else if size >= GB {
        format!("{:.1} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1} KB", size as f64 / KB as f64)
    } else {
        format!("{size} B")
    }
}

/// Format size in human-readable format (abbreviated SI units)
///
/// Used by commands like `ls`, `du`, `stat` for more compact output.
#[allow(
    clippy::cast_precision_loss,
    clippy::as_conversions,
    clippy::float_arithmetic
)]
pub fn format_size_short(size: u64) -> String {
    if size >= PB {
        format!("{:.1}P", size as f64 / PB as f64)
    } else if size >= TB {
        format!("{:.1}T", size as f64 / TB as f64)
    } else if size >= GB {
        format!("{:.1}G", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1}M", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1}K", size as f64 / KB as f64)
    } else {
        format!("{size}B")
    }
}
