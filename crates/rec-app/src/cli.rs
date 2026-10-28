//! Command-line interface argument parsing and batch execution.
//!
//! Conforms to `[SPEC-GUI-001]` and `RULE-I18N-001`.

use std::path::PathBuf;
use rec_core::config::AppConfig;
use rec_engine::traits::CaptureTarget;

/// Command-line argument options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliOptions {
    /// True if recording should immediately commence.
    pub record: bool,
    /// Optional monitor index to capture.
    pub monitor: Option<usize>,
    /// Optional coordinate sub-area formatted as "x,y,width,height".
    pub area: Option<(u32, u32, u32, u32)>,
    /// Destination video file path.
    pub output: Option<PathBuf>,
    /// Recording duration limit in seconds.
    pub duration_secs: Option<u64>,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl CliOptions {
    /// Creates an empty [`CliOptions`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            record: false,
            monitor: None,
            area: None,
            output: None,
            duration_secs: None,
        }
    }

    /// Parses command-line arguments from an iterator of string arguments.
    ///
    /// # Arguments
    ///
    /// * `args` - Iterable string sequence (excluding program name).
    #[must_use]
    pub fn parse_from<I, T>(args: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: AsRef<str>,
    {
        let mut opts = Self::new();
        let mut iter = args.into_iter();

        while let Some(arg) = iter.next() {
            match arg.as_ref() {
                "--record" | "-r" => {
                    opts.record = true;
                }
                "--monitor" | "-m" => {
                    if let Some(val) = iter.next() {
                        opts.monitor = val.as_ref().parse().ok();
                    }
                }
                "--area" | "-a" => {
                    if let Some(val) = iter.next() {
                        let parts: Vec<&str> = val.as_ref().split(',').collect();
                        if parts.len() == 4 {
                            if let (Ok(x), Ok(y), Ok(w), Ok(h)) = (
                                parts[0].parse(),
                                parts[1].parse(),
                                parts[2].parse(),
                                parts[3].parse(),
                            ) {
                                opts.area = Some((x, y, w, h));
                            }
                        }
                    }
                }
                "--output" | "-o" => {
                    if let Some(val) = iter.next() {
                        opts.output = Some(PathBuf::from(val.as_ref()));
                    }
                }
                "--duration" | "-d" => {
                    if let Some(val) = iter.next() {
                        opts.duration_secs = val.as_ref().parse().ok();
                    }
                }
                _ => {}
            }
        }

        opts
    }

    /// Resolves the intended [`CaptureTarget`] from CLI options.
    #[must_use]
    pub fn to_capture_target(&self) -> CaptureTarget {
        if let Some((x, y, width, height)) = self.area {
            CaptureTarget::Area { x, y, width, height }
        } else if let Some(monitor_index) = self.monitor {
            CaptureTarget::FullScreen { monitor_index }
        } else {
            CaptureTarget::FullScreen { monitor_index: 0 }
        }
    }

    /// Applies CLI options into an [`AppConfig`].
    pub fn apply_to_config(&self, config: &mut AppConfig) {
        if let Some(ref out) = self.output {
            if let Some(parent) = out.parent() {
                config.set_output_directory(parent);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing_record_and_area() {
        let args = ["--record", "--area", "10,20,1280,720", "--duration", "60"];
        let opts = CliOptions::parse_from(args);
        assert!(opts.record);
        assert_eq!(opts.area, Some((10, 20, 1280, 720)));
        assert_eq!(opts.duration_secs, Some(60));
    }
}
