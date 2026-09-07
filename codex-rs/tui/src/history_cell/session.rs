//! Session headers, onboarding guidance, and transcript cards.

use super::*;
use crate::line_truncation::line_width;
use crate::line_truncation::truncate_line_with_ellipsis_if_overflow;
use crate::width::display_width;

pub(crate) const SESSION_HEADER_MAX_INNER_WIDTH: usize = 56; // Just an eyeballed value

const LANDING_WIDE_MIN_WIDTH: usize = 52;
const LANDING_MEDIUM_MIN_WIDTH: usize = 36;
// Keep the landing stage close to the viewport edges while leaving a calm gutter around it.
const LANDING_FRAME_MAX_INNER_WIDTH: usize = 104;
const LANDING_MOTION_PERIOD: u8 = 20;
pub(crate) const LANDING_MOTION_STEP: Duration = Duration::from_millis(320);
const LANDING_SWEEP_HALF_WIDTH: usize = 10;
const LANDING_SWEEP_MARGIN: usize = 8;

const LANDING_SILVER_SHADOW: (u8, u8, u8) = (186, 192, 198);
const LANDING_SILVER_MID: (u8, u8, u8) = (194, 200, 205);
const LANDING_SILVER_HIGHLIGHT: (u8, u8, u8) = (202, 207, 212);
const LANDING_SILVER_BRIGHT: Color = Color::Rgb(232, 236, 239);
const LANDING_SILVER_MUTED: Color = Color::Rgb(126, 136, 145);
const LANDING_DARK_GOLD: Color = Color::Rgb(150, 125, 76);
const LANDING_FRAME: Color = Color::Rgb(76, 86, 95);

const LANDING_WORDMARK_WIDE: [[&str; 5]; 8] = [
    ["████████", "████████", "██████  ", "████████", "██    ██"],
    ["████████", "████████", "████████", "████████", "██    ██"],
    ["██      ", "██    ██", "██    ██", "██      ", "  ████  "],
    ["██      ", "██    ██", "██    ██", "██████  ", "   ██   "],
    ["██      ", "██    ██", "██    ██", "██████  ", "   ██   "],
    ["██      ", "██    ██", "██    ██", "██      ", "  ████  "],
    ["████████", "████████", "████████", "████████", "██    ██"],
    ["████████", "████████", "██████  ", "████████", "██    ██"],
];

const LANDING_WORDMARK_MEDIUM: [[&str; 5]; 7] = [
    ["██████", "██████", "████  ", "██████", "██  ██"],
    ["██████", "██████", "██████", "██████", "██  ██"],
    ["██    ", "██  ██", "██  ██", "██    ", " ████ "],
    ["██    ", "██  ██", "██  ██", "█████ ", "  ██  "],
    ["██    ", "██  ██", "██  ██", "██    ", " ████ "],
    ["██████", "██████", "██████", "██████", "██  ██"],
    ["██████", "██████", "████  ", "██████", "██  ██"],
];

pub(crate) fn card_inner_width(width: u16, max_inner_width: usize) -> Option<usize> {
    if width < 4 {
        return None;
    }
    let inner_width = std::cmp::min(width.saturating_sub(4) as usize, max_inner_width);
    Some(inner_width)
}

/// Render `lines` inside a border sized to the widest span in the content.
pub(crate) fn with_border(lines: Vec<Line<'static>>) -> Vec<Line<'static>> {
    with_border_internal(
        lines,
        /*forced_inner_width*/ None,
        Style::default().dim(),
    )
}

/// Render `lines` inside a border whose inner width is at least `inner_width`.
///
/// This is useful when callers have already clamped their content to a
/// specific width and want the border math centralized here instead of
/// duplicating padding logic in the TUI widgets themselves.
pub(crate) fn with_border_with_inner_width(
    lines: Vec<Line<'static>>,
    inner_width: usize,
) -> Vec<Line<'static>> {
    with_border_internal(lines, Some(inner_width), Style::default().dim())
}

fn with_border_internal(
    lines: Vec<Line<'static>>,
    forced_inner_width: Option<usize>,
    border_style: Style,
) -> Vec<Line<'static>> {
    let max_line_width = lines.iter().map(line_width).max().unwrap_or(0);
    let content_width = forced_inner_width
        .unwrap_or(max_line_width)
        .max(max_line_width);

    let mut out = Vec::with_capacity(lines.len() + 2);
    let border_inner_width = content_width + 2;
    out.push(Line::from(Span::styled(
        format!("╭{}╮", "─".repeat(border_inner_width)),
        border_style,
    )));

    for line in lines.into_iter() {
        let used_width = line_width(&line);
        let span_count = line.spans.len();
        let mut spans: Vec<Span<'static>> = Vec::with_capacity(span_count + 4);
        spans.push(Span::styled("│ ", border_style));
        spans.extend(line);
        if used_width < content_width {
            spans.push(Span::styled(
                " ".repeat(content_width - used_width),
                border_style,
            ));
        }
        spans.push(Span::styled(" │", border_style));
        out.push(Line::from(spans));
    }

    out.push(Line::from(Span::styled(
        format!("╰{}╯", "─".repeat(border_inner_width)),
        border_style,
    )));

    out
}

#[derive(Debug)]
struct TooltipHistoryCell {
    tip: String,
    cwd: PathBuf,
}

impl TooltipHistoryCell {
    fn new(tip: String, cwd: &Path) -> Self {
        Self {
            tip,
            cwd: cwd.to_path_buf(),
        }
    }
}

impl HistoryCell for TooltipHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let indent = "  ";
        let indent_width = display_width(indent);
        let wrap_width = usize::from(width.max(1))
            .saturating_sub(indent_width)
            .max(1);
        let mut lines: Vec<Line<'static>> = Vec::new();
        append_markdown(
            &format!("**Tip:** {}", self.tip),
            Some(wrap_width),
            Some(self.cwd.as_path()),
            &mut lines,
        );

        prefix_lines(lines, indent.into(), indent.into())
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        vec![Line::from(format!("Tip: {}", self.tip))]
    }
}

#[derive(Debug)]
pub struct SessionInfoCell(CompositeHistoryCell);

impl HistoryCell for SessionInfoCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        self.0.display_lines(width)
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.0.desired_height(width)
    }

    fn transcript_lines(&self, width: u16) -> Vec<Line<'static>> {
        self.0.transcript_lines(width)
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        self.0.raw_lines()
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "keep local preferences separate while the legacy Config parameter is still required"
)]
pub(crate) fn new_session_info(
    config: &Config,
    local_settings: &crate::local_settings::LocalSettings,
    requested_model: &str,
    session: &ThreadSessionState,
    is_first_event: bool,
    tooltip_override: Option<String>,
    auth_plan: Option<PlanType>,
    show_fast_status: bool,
) -> SessionInfoCell {
    // Header box rendered as history (so it appears at the very top)
    let mut header = SessionHeaderHistoryCell::new(
        session.model.clone(),
        session.reasoning_effort.clone(),
        show_fast_status,
        config.cwd.to_path_buf(),
        CODEX_CLI_VERSION,
    )
    .with_yolo_mode(has_yolo_permissions(
        session.approval_policy,
        &session.permission_profile,
    ))
    .with_landing_plan(auth_plan);
    if is_first_event {
        header = header.with_landing_presentation().with_landing_motion(
            MotionMode::from_animations_enabled(local_settings.tui.animations),
        );
    }
    let mut parts: Vec<Box<dyn HistoryCell>> = vec![Box::new(header)];

    // The landing presentation owns the compact discovery hints so the first screen stays spacious
    // instead of turning into a command directory.
    if !is_first_event {
        if local_settings.tui.show_tooltips
            && let Some(tooltips) = tooltip_override
                .or_else(|| tooltips::get_tooltip(auth_plan, show_fast_status))
                .map(|tip| TooltipHistoryCell::new(tip, &config.cwd))
        {
            parts.push(Box::new(tooltips));
        }
        if requested_model != session.model.as_str() {
            let lines = vec![
                "model changed:".magenta().bold().into(),
                format!("requested: {requested_model}").into(),
                format!("used: {}", session.model).into(),
            ];
            parts.push(Box::new(PlainHistoryCell { lines }));
        }
    }

    SessionInfoCell(CompositeHistoryCell { parts })
}

pub(crate) fn is_yolo_mode(config: &Config) -> bool {
    has_yolo_permissions(
        AskForApproval::from(config.permissions.approval_policy.value()),
        &config.permissions.effective_permission_profile(),
    )
}

pub(crate) fn has_yolo_permissions(
    approval_policy: AskForApproval,
    permission_profile: &PermissionProfile,
) -> bool {
    approval_policy == AskForApproval::Never
        && matches!(
            permission_profile,
            PermissionProfile::Disabled
                | PermissionProfile::Managed {
                    file_system: ManagedFileSystemPermissions::Unrestricted,
                    network: NetworkSandboxPolicy::Enabled,
                }
        )
}
#[derive(Debug)]
pub(crate) struct SessionHeaderHistoryCell {
    version: &'static str,
    model: String,
    model_style: Style,
    reasoning_effort: Option<ReasoningEffortConfig>,
    show_fast_status: bool,
    directory: PathBuf,
    yolo_mode: bool,
    landing_presentation: bool,
    landing_motion_enabled: bool,
    landing_motion_origin: Instant,
    landing_motion_phase_override: Option<u8>,
    landing_plan: Option<String>,
}

impl SessionHeaderHistoryCell {
    pub(crate) fn new(
        model: String,
        reasoning_effort: Option<ReasoningEffortConfig>,
        show_fast_status: bool,
        directory: PathBuf,
        version: &'static str,
    ) -> Self {
        Self::new_with_style(
            model,
            Style::default(),
            reasoning_effort,
            show_fast_status,
            directory,
            version,
        )
    }

    pub(crate) fn new_with_style(
        model: String,
        model_style: Style,
        reasoning_effort: Option<ReasoningEffortConfig>,
        show_fast_status: bool,
        directory: PathBuf,
        version: &'static str,
    ) -> Self {
        Self {
            version,
            model: crate::model_catalog::model_display_name(&model).to_string(),
            model_style,
            reasoning_effort,
            show_fast_status,
            directory,
            yolo_mode: false,
            landing_presentation: false,
            landing_motion_enabled: false,
            landing_motion_origin: Instant::now(),
            landing_motion_phase_override: None,
            landing_plan: None,
        }
    }

    pub(crate) fn with_yolo_mode(mut self, yolo_mode: bool) -> Self {
        self.yolo_mode = yolo_mode;
        self
    }

    pub(crate) fn with_landing_presentation(mut self) -> Self {
        self.landing_presentation = true;
        self
    }

    pub(crate) fn with_landing_motion(mut self, motion_mode: MotionMode) -> Self {
        self.landing_motion_enabled = motion_mode == MotionMode::Animated;
        self
    }

    pub(crate) fn with_landing_plan(mut self, plan: Option<PlanType>) -> Self {
        self.landing_plan = plan
            .filter(|plan| *plan != PlanType::Unknown)
            .map(crate::status::plan_type_display_name);
        self
    }

    #[cfg(test)]
    pub(crate) fn with_landing_motion_phase(mut self, phase: u8) -> Self {
        self.landing_motion_phase_override = Some(phase % LANDING_MOTION_PERIOD);
        self
    }

    pub(crate) fn set_landing_presentation(&mut self, landing_presentation: bool) {
        self.landing_presentation = landing_presentation;
    }

    fn format_directory(&self, max_width: Option<usize>) -> String {
        Self::format_directory_inner(&self.directory, max_width)
    }

    pub(crate) fn format_directory_inner(directory: &Path, max_width: Option<usize>) -> String {
        let formatted = if let Some(rel) = relativize_to_home(directory) {
            if rel.as_os_str().is_empty() {
                "~".to_string()
            } else {
                format!("~{}{}", std::path::MAIN_SEPARATOR, rel.display())
            }
        } else {
            directory.display().to_string()
        };

        if let Some(max_width) = max_width {
            if max_width == 0 {
                return String::new();
            }
            if display_width(formatted.as_str()) > max_width {
                return crate::text_formatting::center_truncate_path(&formatted, max_width);
            }
        }

        formatted
    }

    fn reasoning_label(&self) -> Option<&str> {
        self.reasoning_effort
            .as_ref()
            .map(ReasoningEffortConfig::as_str)
    }

    fn centered_line(mut line: Line<'static>, width: usize) -> Line<'static> {
        let padding = width.saturating_sub(line_width(&line)) / 2;
        if padding > 0 {
            line.spans.insert(0, " ".repeat(padding).into());
        }
        line
    }

    fn landing_wordmark_lines(
        wordmark: &[[&'static str; 5]],
        letter_spacing: &'static str,
        width: usize,
        motion_phase: u8,
        motion_active: bool,
    ) -> Vec<Line<'static>> {
        let wordmark_width = wordmark
            .first()
            .map(|row| {
                row.iter().map(|glyph| display_width(glyph)).sum::<usize>()
                    + display_width(letter_spacing) * row.len().saturating_sub(1)
            })
            .unwrap_or(0);
        let sweep_travel = wordmark_width + 2 * (LANDING_SWEEP_HALF_WIDTH + LANDING_SWEEP_MARGIN);
        let sweep_period = usize::from(LANDING_MOTION_PERIOD);
        let sweep_center = (usize::from(motion_phase) * sweep_travel / sweep_period) as isize
            - (LANDING_SWEEP_HALF_WIDTH + LANDING_SWEEP_MARGIN) as isize;

        wordmark
            .iter()
            .enumerate()
            .map(|(row_index, row)| {
                let mut spans = Vec::with_capacity(wordmark_width + row.len());
                let mut character_x = 0;
                for (letter_index, glyph) in row.iter().enumerate() {
                    if letter_index > 0 {
                        spans.push(letter_spacing.into());
                        character_x += display_width(letter_spacing);
                    }
                    for character in glyph.chars() {
                        // Keep the material deterministic and tied to the wordmark's global
                        // character coordinate. The small ordered variation reads as brushed
                        // metal without introducing flicker or per-frame noise.
                        let edge_distance = row_index.min(wordmark.len() - row_index - 1);
                        let band_tone = match edge_distance {
                            0 => LANDING_SILVER_HIGHLIGHT,
                            1 => LANDING_SILVER_MID,
                            2 => LANDING_SILVER_SHADOW,
                            _ => LANDING_SILVER_MID,
                        };
                        let brushed_tone = if (character_x + row_index * 3) % 7 == 0 {
                            -1
                        } else {
                            0
                        };
                        let sweep_distance = (character_x as isize - sweep_center).unsigned_abs();
                        let sweep_tone = if motion_active {
                            LANDING_SWEEP_HALF_WIDTH
                                .saturating_sub(sweep_distance)
                                .saturating_mul(20)
                                / LANDING_SWEEP_HALF_WIDTH
                        } else {
                            0
                        };
                        let (red, green, blue) = band_tone;
                        let luminance =
                            (i16::from(red) + brushed_tone + sweep_tone as i16).clamp(0, 255) as u8;
                        let luminance_delta = i16::from(luminance) - i16::from(red);
                        let scan_crosses_notch = motion_active
                            && sweep_distance <= 1
                            && brushed_tone < 0
                            && character != ' ';
                        let color = if scan_crosses_notch {
                            Color::Rgb(
                                luminance,
                                luminance.saturating_sub(8),
                                luminance.saturating_sub(22),
                            )
                        } else {
                            Color::Rgb(
                                luminance,
                                (i16::from(green) + luminance_delta).clamp(0, 255) as u8,
                                (i16::from(blue) + luminance_delta).clamp(0, 255) as u8,
                            )
                        };
                        let style = Style::default().fg(color).bold();
                        spans.push(Span::styled(character.to_string(), style));
                        character_x += 1;
                    }
                }
                let mut line = Self::centered_line(Line::from(spans), width);
                for span in line.spans.iter_mut().rev() {
                    let trimmed = span.content.trim_end().to_owned();
                    let was_non_empty = !trimmed.is_empty();
                    span.content = trimmed.into();
                    if was_non_empty {
                        break;
                    }
                }
                line
            })
            .collect()
    }

    fn landing_display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let width = usize::from(width);
        if width == 0 {
            return Vec::new();
        }

        let preferred_inner_width = width.saturating_sub(8).min(LANDING_FRAME_MAX_INNER_WIDTH);
        if preferred_inner_width == 0 {
            return Vec::new();
        }
        let available_inner_width = width.saturating_sub(4).min(LANDING_FRAME_MAX_INNER_WIDTH);
        let wide_hints_width = display_width("/ commands      @ files      ? shortcuts");
        let compact_hints_width = display_width("/ commands    ? shortcuts");
        let short_hints_width = display_width("? shortcuts");
        let frame_inner_width = if available_inner_width >= wide_hints_width {
            preferred_inner_width.max(wide_hints_width)
        } else if available_inner_width >= compact_hints_width {
            preferred_inner_width.max(compact_hints_width)
        } else if available_inner_width >= short_hints_width {
            preferred_inner_width.max(short_hints_width)
        } else {
            preferred_inner_width
        };
        let motion_phase = self.landing_motion_phase_override.unwrap_or_else(|| {
            if self.landing_motion_enabled {
                ((self.landing_motion_origin.elapsed().as_millis()
                    / LANDING_MOTION_STEP.as_millis())
                    % u128::from(LANDING_MOTION_PERIOD)) as u8
            } else {
                0
            }
        });
        let mut lines = vec![Line::from("")];
        if frame_inner_width >= LANDING_WIDE_MIN_WIDTH {
            lines.extend(Self::landing_wordmark_lines(
                &LANDING_WORDMARK_WIDE,
                "  ",
                frame_inner_width,
                motion_phase,
                self.landing_motion_enabled || self.landing_motion_phase_override.is_some(),
            ));
            lines.push(Line::from(""));
        } else if frame_inner_width >= LANDING_MEDIUM_MIN_WIDTH {
            lines.extend(Self::landing_wordmark_lines(
                &LANDING_WORDMARK_MEDIUM,
                " ",
                frame_inner_width,
                motion_phase,
                self.landing_motion_enabled || self.landing_motion_phase_override.is_some(),
            ));
            lines.push(Line::from(""));
        } else {
            let wordmark = if frame_inner_width >= 9 {
                "C O D E X"
            } else {
                "CODEX"
            };
            let wordmark = truncate_line_with_ellipsis_if_overflow(
                Line::from(wordmark).style(Style::default().fg(LANDING_SILVER_BRIGHT).bold()),
                frame_inner_width,
            );
            lines.push(Self::centered_line(wordmark, frame_inner_width));
            lines.push(Line::from(""));
        }

        // Only show values available at bootstrap. Usage is intentionally absent until account
        // limits arrive; model and cwd are already repeated in the footer.
        if frame_inner_width >= 20 {
            let mut index_rows = Vec::new();
            if let Some(plan) = &self.landing_plan {
                index_rows.push(("PLAN", plan.clone()));
            }
            if self.yolo_mode {
                index_rows.push(("MODE", "YOLO".to_string()));
            }
            let index_width = index_rows
                .iter()
                .map(|(_, value)| 12 + display_width(value))
                .max()
                .unwrap_or(0);
            let index_padding = frame_inner_width.saturating_sub(index_width) / 2;
            for (label, value) in index_rows {
                let index_line = Line::from(vec![
                    " ".repeat(index_padding).into(),
                    Span::styled(
                        format!("§ {label:<4}"),
                        Style::default().fg(LANDING_DARK_GOLD).bold(),
                    ),
                    "      ".into(),
                    Span::styled(value, Style::default().fg(LANDING_SILVER_BRIGHT)),
                ]);
                lines.push(truncate_line_with_ellipsis_if_overflow(
                    index_line,
                    frame_inner_width,
                ));
            }
        }

        lines.push(Line::from(""));
        let hints = if frame_inner_width >= wide_hints_width {
            vec![
                Span::styled("/", Style::default().fg(LANDING_DARK_GOLD).bold()),
                Span::styled(" commands", Style::default().fg(LANDING_SILVER_MUTED)),
                "      ".into(),
                Span::styled("@", Style::default().fg(LANDING_DARK_GOLD).bold()),
                Span::styled(" files", Style::default().fg(LANDING_SILVER_MUTED)),
                "      ".into(),
                Span::styled("?", Style::default().fg(LANDING_DARK_GOLD).bold()),
                Span::styled(" shortcuts", Style::default().fg(LANDING_SILVER_MUTED)),
            ]
        } else if frame_inner_width >= compact_hints_width {
            vec![
                Span::styled("/", Style::default().fg(LANDING_DARK_GOLD).bold()),
                Span::styled(" commands", Style::default().fg(LANDING_SILVER_MUTED)),
                "    ".into(),
                Span::styled("?", Style::default().fg(LANDING_DARK_GOLD).bold()),
                Span::styled(" shortcuts", Style::default().fg(LANDING_SILVER_MUTED)),
            ]
        } else {
            vec![
                Span::styled("?", Style::default().fg(LANDING_DARK_GOLD).bold()),
                Span::styled(" shortcuts", Style::default().fg(LANDING_SILVER_MUTED)),
            ]
        };
        let hints = truncate_line_with_ellipsis_if_overflow(Line::from(hints), frame_inner_width);
        lines.push(Self::centered_line(hints, frame_inner_width));
        lines.push(Line::from(""));

        let border_inner_width = frame_inner_width + 2;
        let frame_width = frame_inner_width + 4;
        let frame_style = Style::default().fg(LANDING_FRAME);
        let marker = "§ CODEX";
        let marker_width = display_width(marker);
        let mut top = vec![Span::styled("╭─", frame_style)];
        if frame_width >= marker_width + 11 {
            top.push(Span::styled("┬ ", frame_style));
            top.push(Span::styled(
                marker,
                Style::default().fg(LANDING_DARK_GOLD).bold(),
            ));
            top.push(Span::styled(" ", frame_style));
            let used_width = 2 + 2 + marker_width + 1;
            top.push(Span::styled(
                "─".repeat(frame_width.saturating_sub(used_width + 2)),
                frame_style,
            ));
            top.push(Span::styled("┬╮", frame_style));
        } else {
            top.push(Span::styled(
                "─".repeat(border_inner_width.saturating_sub(1)),
                frame_style,
            ));
            top.push(Span::styled("╮", frame_style));
        }

        let mut framed = Vec::with_capacity(lines.len() + 2);
        framed.push(Line::from(top));
        for line in lines {
            let used_width = line_width(&line);
            let mut spans = Vec::with_capacity(line.spans.len() + 4);
            spans.push(Span::styled("│ ", frame_style));
            spans.extend(line);
            if used_width < frame_inner_width {
                spans.push(Span::styled(
                    " ".repeat(frame_inner_width - used_width),
                    frame_style,
                ));
            }
            spans.push(Span::styled(" │", frame_style));
            framed.push(Line::from(spans));
        }
        let bottom = if frame_width >= marker_width + 11 {
            Line::from(vec![
                Span::styled("╰─┴", frame_style),
                Span::styled("─".repeat(frame_width.saturating_sub(6)), frame_style),
                Span::styled("┴─╯", frame_style),
            ])
        } else {
            Line::from(Span::styled(
                format!("╰{}╯", "─".repeat(border_inner_width)),
                frame_style,
            ))
        };
        framed.push(bottom);
        framed
            .into_iter()
            .map(|line| Self::centered_line(line, width))
            .collect()
    }
}

impl HistoryCell for SessionHeaderHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        if self.landing_presentation {
            return self.landing_display_lines(width);
        }

        let Some(inner_width) = card_inner_width(width, SESSION_HEADER_MAX_INNER_WIDTH) else {
            return Vec::new();
        };

        let make_row = |spans: Vec<Span<'static>>| Line::from(spans);

        // Title line rendered inside the box: ">_ OpenAI Codex (vX)"
        let title_spans: Vec<Span<'static>> = vec![
            Span::from(">_ ").dim(),
            Span::from("OpenAI Codex").bold(),
            Span::from(" ").dim(),
            Span::from(format!("(v{})", self.version)).dim(),
        ];

        const CHANGE_MODEL_HINT_COMMAND: &str = "/model";
        const CHANGE_MODEL_HINT_EXPLANATION: &str = " to change";
        const DIR_LABEL: &str = "directory:";
        const PERMISSIONS_LABEL: &str = "permissions:";
        let label_width = if self.yolo_mode {
            DIR_LABEL.len().max(PERMISSIONS_LABEL.len())
        } else {
            DIR_LABEL.len()
        };

        let model_label = format!(
            "{model_label:<label_width$}",
            model_label = "model:",
            label_width = label_width
        );
        let reasoning_label = self.reasoning_label();
        let model_spans: Vec<Span<'static>> = {
            let mut spans = vec![
                Span::from(format!("{model_label} ")).dim(),
                Span::styled(self.model.clone(), self.model_style),
            ];
            if let Some(reasoning) = reasoning_label {
                spans.push(Span::from(" "));
                spans.push(Span::from(reasoning.to_owned()));
            }
            if self.show_fast_status {
                spans.push("   ".into());
                spans.push(Span::styled("fast", self.model_style.magenta()));
            }
            spans.push("   ".dim());
            spans.push(CHANGE_MODEL_HINT_COMMAND.cyan());
            spans.push(CHANGE_MODEL_HINT_EXPLANATION.dim());
            spans
        };

        let dir_label = format!("{DIR_LABEL:<label_width$}");
        let dir_prefix = format!("{dir_label} ");
        let dir_prefix_width = display_width(dir_prefix.as_str());
        let dir_max_width = inner_width.saturating_sub(dir_prefix_width);
        let dir = self.format_directory(Some(dir_max_width));
        let dir_spans = vec![Span::from(dir_prefix).dim(), Span::from(dir)];

        let mut lines = vec![
            make_row(title_spans),
            make_row(Vec::new()),
            make_row(model_spans),
            make_row(dir_spans),
        ];

        if self.yolo_mode {
            let permissions_label = format!("{PERMISSIONS_LABEL:<label_width$}");
            lines.push(make_row(vec![
                Span::from(format!("{permissions_label} ")).dim(),
                "YOLO mode".magenta().bold(),
            ]));
        }

        let lines = lines
            .into_iter()
            .map(|line| truncate_line_with_ellipsis_if_overflow(line, inner_width))
            .collect();
        with_border(lines)
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        let mut lines = vec![
            Line::from(format!("OpenAI Codex (v{})", self.version)),
            Line::from(format!(
                "model: {}{}",
                self.model,
                self.reasoning_label()
                    .map(|reasoning| format!(" {reasoning}"))
                    .unwrap_or_default()
            )),
            Line::from(format!(
                "directory: {}",
                self.format_directory(/*max_width*/ None)
            )),
        ];
        if self.yolo_mode {
            lines.push(Line::from("permissions: YOLO mode"));
        }
        lines
    }
}
