use crate::color;
use crate::color::rgb;
use ratatui::prelude::*;

pub fn user_color() -> Color {
    rgb(138, 180, 248)
}
pub fn ai_color() -> Color {
    rgb(129, 199, 132)
}
pub fn tool_color() -> Color {
    rgb(120, 120, 120)
}
pub fn file_link_color() -> Color {
    rgb(180, 200, 255)
}
pub fn dim_color() -> Color {
    rgb(80, 80, 80)
}
pub fn accent_color() -> Color {
    rgb(186, 139, 255)
}
pub fn system_message_color() -> Color {
    rgb(255, 170, 220)
}
pub fn queued_color() -> Color {
    rgb(255, 193, 7)
}
pub fn asap_color() -> Color {
    rgb(110, 210, 255)
}
pub fn pending_color() -> Color {
    rgb(140, 140, 140)
}
pub fn user_text() -> Color {
    rgb(245, 245, 255)
}
pub fn user_bg() -> Color {
    rgb(35, 40, 50)
}
pub fn ai_text() -> Color {
    rgb(220, 220, 215)
}
pub fn header_icon_color() -> Color {
    rgb(120, 210, 230)
}
pub fn header_name_color() -> Color {
    rgb(190, 210, 235)
}
pub fn header_session_color() -> Color {
    rgb(255, 255, 255)
}

// Spinner frames for animated status
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const STATIC_ACTIVITY_INDICATOR: &str = "•";
const TOOL_ACTIVITY_WIDTH: usize = 5;

fn safe_animation_fps(fps: f32) -> f32 {
    if fps.is_finite() && fps > 0.0 {
        fps.min(120.0)
    } else {
        1.0
    }
}

pub fn spinner_frame_index(elapsed: f32, fps: f32) -> usize {
    let elapsed = if elapsed.is_finite() {
        elapsed.max(0.0)
    } else {
        0.0
    };
    ((elapsed * safe_animation_fps(fps)) as usize) % SPINNER_FRAMES.len()
}

pub fn spinner_frame(elapsed: f32, fps: f32) -> &'static str {
    SPINNER_FRAMES[spinner_frame_index(elapsed, fps)]
}

pub fn activity_indicator_frame_index(
    elapsed: f32,
    fps: f32,
    enable_decorative_animations: bool,
) -> usize {
    if enable_decorative_animations {
        spinner_frame_index(elapsed, fps)
    } else {
        0
    }
}

pub fn activity_indicator(
    elapsed: f32,
    fps: f32,
    enable_decorative_animations: bool,
) -> &'static str {
    if enable_decorative_animations {
        spinner_frame(elapsed, fps)
    } else {
        STATIC_ACTIVITY_INDICATOR
    }
}

pub fn tool_activity_bars(elapsed: f32, enable_decorative_animations: bool) -> (String, String) {
    if !enable_decorative_animations {
        let idle = "·".repeat(TOOL_ACTIVITY_WIDTH);
        return (idle.clone(), idle);
    }

    let elapsed = if elapsed.is_finite() {
        elapsed.max(0.0)
    } else {
        0.0
    };
    let head = ((elapsed * 8.0) as usize) % TOOL_ACTIVITY_WIDTH;

    let build = |mirror: bool| -> String {
        (0..TOOL_ACTIVITY_WIDTH)
            .map(|i| {
                let i = if mirror {
                    TOOL_ACTIVITY_WIDTH - 1 - i
                } else {
                    i
                };
                let dist = (TOOL_ACTIVITY_WIDTH + i - head) % TOOL_ACTIVITY_WIDTH;
                match dist {
                    0 => '●',
                    1 | 4 => '◆',
                    _ => '·',
                }
            })
            .collect()
    };

    (build(false), build(true))
}

/// Convert HSL to RGB (h in 0-360, s and l in 0-1)
/// Chroma color based on position and time - creates flowing rainbow wave
/// Calculate chroma color with fade-in from dim during startup
/// Calculate smooth animated color for the header (single color, no position)
pub fn color_to_floats(c: Color, fallback: (f32, f32, f32)) -> (f32, f32, f32) {
    match c {
        Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
        Color::Indexed(n) => {
            let (r, g, b) = color::indexed_to_rgb(n);
            (r as f32, g as f32, b as f32)
        }
        _ => fallback,
    }
}

pub fn blend_color(from: Color, to: Color, t: f32) -> Color {
    let (fr, fg, fb) = color_to_floats(from, (80.0, 80.0, 80.0));
    let (tr, tg, tb) = color_to_floats(to, (200.0, 200.0, 200.0));
    let r = fr + (tr - fr) * t;
    let g = fg + (tg - fg) * t;
    let b = fb + (tb - fb) * t;
    rgb(
        r.clamp(0.0, 255.0) as u8,
        g.clamp(0.0, 255.0) as u8,
        b.clamp(0.0, 255.0) as u8,
    )
}

pub fn rainbow_prompt_color(distance: usize) -> Color {
    // Rainbow colors (hue progression): red -> orange -> yellow -> green -> cyan -> blue -> violet
    const RAINBOW: [(u8, u8, u8); 7] = [
        (255, 80, 80),   // Red (softened)
        (255, 160, 80),  // Orange
        (255, 230, 80),  // Yellow
        (80, 220, 100),  // Green
        (80, 200, 220),  // Cyan
        (100, 140, 255), // Blue
        (180, 100, 255), // Violet
    ];

    // Gray target (dim_color())
    const GRAY: (u8, u8, u8) = (80, 80, 80);

    // Exponential decay factor - how quickly we fade to gray
    // decay = e^(-distance * rate), rate of ~0.4 gives nice falloff
    let decay = (-0.4 * distance as f32).exp();

    // Select rainbow color based on distance (cycle through)
    let rainbow_idx = distance.min(RAINBOW.len() - 1);
    let (r, g, b) = RAINBOW[rainbow_idx];

    // Blend rainbow color with gray based on decay
    // At distance 0: 100% rainbow, as distance increases: approaches gray
    let blend = |rainbow: u8, gray: u8| -> u8 {
        (rainbow as f32 * decay + gray as f32 * (1.0 - decay)) as u8
    };

    rgb(blend(r, GRAY.0), blend(g, GRAY.1), blend(b, GRAY.2))
}

pub fn prompt_entry_color(base: Color, t: f32) -> Color {
    let peak = rgb(255, 230, 120);
    // Quick pulse in/out over the animation window.
    let phase = if t < 0.5 { t * 2.0 } else { (1.0 - t) * 2.0 };
    blend_color(base, peak, phase.clamp(0.0, 1.0) * 0.7)
}

pub fn prompt_entry_bg_color(base: Color, t: f32) -> Color {
    let spotlight = rgb(58, 66, 82);
    let ease_in = 1.0 - (1.0 - t).powi(3);
    let ease_out = (1.0 - t).powi(2);
    let phase = (ease_in * ease_out * 1.65).clamp(0.0, 1.0);
    blend_color(base, spotlight, phase * 0.85)
}

pub fn prompt_entry_shimmer_color(base: Color, pos: f32, t: f32) -> Color {
    let travel = (t * 1.15).clamp(0.0, 1.0);
    let width = 0.18;
    let dist = (pos - travel).abs();
    let shimmer = (1.0 - (dist / width).clamp(0.0, 1.0)).powf(2.2);
    let pulse = (1.0 - t).powf(0.55);
    let highlight = rgb(255, 248, 210);
    blend_color(base, highlight, shimmer * pulse * 0.7)
}

/// Generate an animated color that pulses between two colors
pub fn animated_tool_color(elapsed: f32, enable_decorative_animations: bool) -> Color {
    if !enable_decorative_animations {
        return tool_color();
    }

    // Cycle period of ~1.5 seconds
    let t = (elapsed * 2.0).sin() * 0.5 + 0.5; // 0.0 to 1.0

    // Interpolate between cyan and purple
    let r = (80.0 + t * 106.0) as u8; // 80 -> 186
    let g = (200.0 - t * 61.0) as u8; // 200 -> 139
    let b = (220.0 + t * 35.0) as u8; // 220 -> 255

    rgb(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spinner_frame_index_tolerates_invalid_timing_inputs() {
        assert_eq!(spinner_frame_index(f32::NAN, 12.5), 0);
        assert_eq!(spinner_frame_index(1.0, f32::NAN), 1);
        assert_eq!(spinner_frame_index(-10.0, 12.5), 0);
        assert!(spinner_frame_index(10_000.0, 10_000.0) < SPINNER_FRAMES.len());
    }

    #[test]
    fn activity_indicator_respects_reduced_motion() {
        assert_eq!(
            activity_indicator(10.0, 12.5, false),
            STATIC_ACTIVITY_INDICATOR
        );
        assert_eq!(activity_indicator_frame_index(10.0, 12.5, false), 0);
    }

    #[test]
    fn tool_activity_bars_are_stable_and_symmetric() {
        let (left, right) = tool_activity_bars(0.0, true);
        assert_eq!(left.chars().count(), TOOL_ACTIVITY_WIDTH);
        assert_eq!(right.chars().count(), TOOL_ACTIVITY_WIDTH);
        assert!(left.contains('●'));
        assert_eq!(right, left.chars().rev().collect::<String>());

        let (reduced_left, reduced_right) = tool_activity_bars(0.0, false);
        assert_eq!(reduced_left, "·".repeat(TOOL_ACTIVITY_WIDTH));
        assert_eq!(reduced_right, reduced_left);
    }
}
