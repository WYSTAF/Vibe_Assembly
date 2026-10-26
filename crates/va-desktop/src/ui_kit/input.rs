//! Native text-service bridge, shared by the composer and palette.
use crate::{state::Workbench, theme};
use gpui::prelude::*;
use gpui::{
    canvas, Bounds, Context, ElementInputHandler, Entity, EntityInputHandler, FocusHandle,
    HighlightStyle, Pixels, Point, Rgba, StyledText, UTF16Selection, Window,
};
use std::ops::Range;

#[derive(Default)]
pub struct InputSelection {
    pub anchor: usize,
    pub head: usize,
    marked: Option<Range<usize>>,
}

fn boundary(text: &str, offset: usize) -> usize {
    let mut offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

fn from_utf16(text: &str, offset: usize) -> usize {
    let mut units = 0;
    for (byte, ch) in text.char_indices() {
        if units + ch.len_utf16() > offset {
            return byte;
        }
        units += ch.len_utf16();
    }
    text.len()
}

fn to_utf16(text: &str, range: Range<usize>) -> Range<usize> {
    text[..boundary(text, range.start)].encode_utf16().count()
        ..text[..boundary(text, range.end)].encode_utf16().count()
}

impl InputSelection {
    pub fn range(&self, text: &str) -> Range<usize> {
        boundary(text, self.anchor.min(self.head))..boundary(text, self.anchor.max(self.head))
    }

    pub fn replace(
        &mut self,
        text: &mut String,
        range: Option<Range<usize>>,
        replacement: &str,
    ) -> bool {
        let range = range
            .map(|range| from_utf16(text, range.start)..from_utf16(text, range.end))
            .or_else(|| self.marked.clone())
            .unwrap_or_else(|| self.range(text));
        let range = boundary(text, range.start)..boundary(text, range.end);
        if range.start > range.end || text.len() - range.len() + replacement.len() > 4_000 {
            return false;
        }
        text.replace_range(range.clone(), replacement);
        self.head = range.start + replacement.len();
        self.anchor = self.head;
        self.marked = None;
        true
    }

    pub fn key(&mut self, text: &mut String, key: &str, shift: bool) -> bool {
        let range = self.range(text);
        match key {
            "backspace" | "delete" => {
                if range.is_empty() {
                    if key == "backspace" {
                        self.anchor = text[..range.start]
                            .char_indices()
                            .last()
                            .map_or(0, |(i, _)| i);
                    } else {
                        self.head = text[range.end..]
                            .chars()
                            .next()
                            .map_or(range.end, |ch| range.end + ch.len_utf8());
                    }
                }
                self.replace(text, None, "");
            }
            "left" | "right" | "home" | "end" => {
                let cursor = boundary(text, self.head);
                self.head = match key {
                    "home" => text[..cursor].rfind('\n').map_or(0, |i| i + 1),
                    "end" => text[cursor..].find('\n').map_or(text.len(), |i| cursor + i),
                    "left" if !shift && !range.is_empty() => range.start,
                    "right" if !shift && !range.is_empty() => range.end,
                    "left" => text[..cursor].char_indices().last().map_or(0, |(i, _)| i),
                    _ => text[cursor..]
                        .chars()
                        .next()
                        .map_or(cursor, |ch| cursor + ch.len_utf8()),
                };
                if !shift {
                    self.anchor = self.head;
                }
                self.marked = None;
            }
            _ => return false,
        }
        true
    }

    pub fn display(&self, text: &str, focused: bool, placeholder: &str, accent: Rgba) -> gpui::Div {
        let range = self.range(text);
        let value = if text.is_empty() {
            placeholder.to_string()
        } else if focused && range.is_empty() {
            format!("{}▏{}", &text[..range.start], &text[range.end..])
        } else {
            text.to_string()
        };
        let highlights = if focused && !range.is_empty() {
            vec![(
                range,
                HighlightStyle {
                    background_color: Some(theme::wash(accent, 0.20)),
                    ..Default::default()
                },
            )]
        } else {
            Vec::new()
        };
        gpui::div()
            .text_color(if text.is_empty() {
                theme::text_muted()
            } else {
                theme::text_primary()
            })
            .child(StyledText::new(value).with_highlights(highlights))
    }
}

pub fn capture(entity: Entity<Workbench>, focus: FocusHandle) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, cx| {
            window.handle_input(&focus, ElementInputHandler::new(bounds, entity.clone()), cx);
        },
    )
    .absolute()
    .size_full()
}

impl Workbench {
    fn input(&self) -> (&str, &InputSelection) {
        if self.state.show_palette {
            (&self.state.palette_query, &self.palette_selection)
        } else {
            (&self.state.prompt_input, &self.prompt_selection)
        }
    }
    pub fn edit_input(&mut self, edit: impl FnOnce(&mut String, &mut InputSelection)) {
        if self.state.show_palette {
            let previous = self.state.palette_query.clone();
            edit(&mut self.state.palette_query, &mut self.palette_selection);
            if previous != self.state.palette_query {
                self.state.palette_index = 0;
            }
        } else {
            edit(&mut self.state.prompt_input, &mut self.prompt_selection);
        }
    }
    pub fn text_shortcut(&mut self, key: &gpui::Keystroke, cx: &mut Context<Self>) -> bool {
        if key.modifiers.control {
            match key.key.as_str() {
                "a" => self.edit_input(|text, selection| {
                    selection.anchor = 0;
                    selection.head = text.len();
                }),
                "v" => {
                    if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                        self.edit_input(|value, selection| {
                            selection.replace(value, None, &text);
                        });
                    }
                }
                "c" | "x" => {
                    let (text, selection) = self.input();
                    let selected = text[selection.range(text)].to_string();
                    if !selected.is_empty() {
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(selected));
                    }
                    if key.key == "x" {
                        self.edit_input(|text, selection| {
                            selection.replace(text, None, "");
                        });
                    }
                }
                _ => return false,
            }
            true
        } else if !key.modifiers.alt && !key.modifiers.platform {
            let mut handled = false;
            self.edit_input(|text, selection| {
                handled = selection.key(text, &key.key, key.modifiers.shift);
            });
            handled
        } else {
            false
        }
    }
}

impl EntityInputHandler for Workbench {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        actual: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let (text, _) = self.input();
        let range = from_utf16(text, range.start)..from_utf16(text, range.end);
        actual.replace(to_utf16(text, range.clone()));
        Some(text[range].to_string())
    }
    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let (text, selection) = self.input();
        Some(UTF16Selection {
            range: to_utf16(text, selection.range(text)),
            reversed: selection.head < selection.anchor,
        })
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        let (text, selection) = self.input();
        selection.marked.clone().map(|range| to_utf16(text, range))
    }
    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.edit_input(|_, selection| selection.marked = None);
    }
    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = if self.state.show_palette {
            text.replace(['\n', '\r'], " ")
        } else {
            text.to_string()
        };
        self.edit_input(|value, selection| {
            selection.replace(value, range, &text);
        });
        cx.notify();
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.edit_input(|value, selection| {
            let start = range
                .as_ref()
                .map(|range| from_utf16(value, range.start))
                .or_else(|| selection.marked.as_ref().map(|range| range.start))
                .unwrap_or_else(|| selection.range(value).start);
            if selection.replace(value, range, text) {
                selection.marked = (!text.is_empty()).then_some(start..start + text.len());
                if let Some(selected) = selected {
                    selection.anchor = start + from_utf16(text, selected.start);
                    selection.head = start + from_utf16(text, selected.end);
                }
            }
        });
        cx.notify();
    }
    fn bounds_for_range(
        &mut self,
        _: Range<usize>,
        bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        Some(bounds)
    }
    fn character_index_for_point(
        &mut self,
        _: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_selection_replace_and_backspace_respect_utf16_and_utf8() {
        let mut text = "A🦀سلامZ".to_string();
        let mut selection = InputSelection::default();
        assert!(selection.replace(&mut text, Some(1..3), "界"));
        assert_eq!(text, "A界سلامZ");
        selection.key(&mut text, "backspace", false);
        assert_eq!(text, "AسلامZ");
        selection.anchor = 0;
        selection.head = text.len();
        selection.replace(&mut text, None, "new\nline");
        assert_eq!(text, "new\nline");
        selection.key(&mut text, "home", false);
        assert_eq!(selection.head, 4);
        selection.key(&mut text, "right", true);
        selection.replace(&mut text, None, "L");
        assert_eq!(text, "new\nLine");
    }
    #[test]
    fn replacement_limits_are_atomic_and_stale_selections_clamp() {
        let mut text = "ok".into();
        let mut selection = InputSelection {
            anchor: 99,
            head: 99,
            marked: None,
        };
        assert!(!selection.replace(&mut text, None, &"x".repeat(4001)));
        assert_eq!(text, "ok");
        assert!(selection.replace(&mut text, None, "!"));
        assert_eq!(text, "ok!");
        selection.marked = Some(1..200);
        text.clear();
        assert!(selection.replace(&mut text, None, "fresh"));
        assert_eq!(text, "fresh");
        assert_eq!(from_utf16("🦀", 1), 0);
    }
}
