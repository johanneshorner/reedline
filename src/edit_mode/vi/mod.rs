mod command;
mod motion;
mod parser;
mod vi_keybindings;

use std::collections::HashMap;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
pub use vi_keybindings::{default_vi_insert_keybindings, default_vi_normal_keybindings};

use self::motion::ViCharSearch;

use super::{
    keybindings::{
        to_lowercase_key_code, KeyNode, KeySequenceResult, PartialKeySequence, Sequence,
    },
    EditMode, KeyCombination,
};
use crate::{
    edit_mode::{keybindings::Keybindings, vi::parser::parse},
    enums::{EditCommand, ReedlineEvent, ReedlineRawEvent},
    LineBuffer, PromptEditMode, PromptViMode,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum ViMode {
    Normal,
    Insert,
    Visual,
}

/// This parses incoming input `Event`s like a Vi-Style editor
pub struct Vi {
    cache: Vec<char>,
    insert_keybindings: Keybindings,
    normal_keybindings: Keybindings,
    mode: ViMode,
    previous: Option<ReedlineEvent>,
    // last f, F, t, T motion for ; and ,
    last_char_search: Option<ViCharSearch>,
    partial_key_sequence: Option<PartialKeySequence>,
}

impl Default for Vi {
    fn default() -> Self {
        Vi {
            insert_keybindings: default_vi_insert_keybindings(),
            normal_keybindings: default_vi_normal_keybindings(),
            cache: Vec::new(),
            mode: ViMode::Insert,
            previous: None,
            last_char_search: None,
            partial_key_sequence: None,
        }
    }
}

impl Vi {
    /// Creates Vi editor using defined keybindings
    pub fn new(insert_keybindings: Keybindings, normal_keybindings: Keybindings) -> Self {
        Self {
            insert_keybindings,
            normal_keybindings,
            ..Default::default()
        }
    }

    fn active_bindings(&self) -> &Keybindings {
        match self.mode {
            ViMode::Normal => &self.normal_keybindings,
            ViMode::Visual => &self.normal_keybindings,
            ViMode::Insert => &self.insert_keybindings,
        }
    }

    fn handle_binding(&mut self, kc: KeyCombination) -> Option<ReedlineEvent> {
        let Some(mut partial_key_sequence) = self.partial_key_sequence.take().or_else(|| {
            self.active_bindings()
                .find_binding(kc.modifier, to_lowercase_key_code(kc.key_code))
                .map(|key_node| {
                    PartialKeySequence::new(match key_node {
                        KeyNode::Sequence(sequence) => sequence,
                        KeyNode::Event(reedline_event) => Sequence {
                            // TODO: really, really, REALLY hacky
                            map: HashMap::from([(kc.clone(), KeyNode::Event(reedline_event))]),
                        },
                    })
                })
        }) else {
            return if let (ViMode::Insert, KeyCode::Char(c)) = (self.mode, kc.key_code) {
                Some(ReedlineEvent::Edit(vec![EditCommand::InsertChar(c)]))
            } else {
                None
            };
        };

        match partial_key_sequence.advance(kc) {
            KeySequenceResult::Pending => None,
            KeySequenceResult::Matched(reedline_event) => Some(reedline_event),
            KeySequenceResult::Cancelled(keycombinations) => {
                let mut events = vec![];
                for kc in keycombinations {
                    if let KeyCode::Char(c) = kc.key_code {
                        events.push(ReedlineEvent::Edit(vec![EditCommand::InsertChar(c)]))
                    } else {
                        match self
                            .active_bindings()
                            .find_binding(kc.modifier, to_lowercase_key_code(kc.key_code))
                        {
                            Some(KeyNode::Event(event)) => events.push(event),
                            Some(KeyNode::Sequence(_)) => unreachable!(""),
                            None => {}
                        }
                    }
                }
                Some(ReedlineEvent::Multiple(events))
            }
        }
    }
}

impl EditMode for Vi {
    fn parse_event(&mut self, _line_buffer: &LineBuffer, event: ReedlineRawEvent) -> ReedlineEvent {
        match event.into() {
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => match (self.mode, modifiers, code) {
                (ViMode::Normal | ViMode::Visual, modifier, KeyCode::Char(c)) => {
                    let c = c.to_ascii_lowercase();

                    if let Some(event) = self.handle_binding(KeyCombination {
                        modifier: modifiers,
                        key_code: KeyCode::Char(c),
                    }) {
                        event
                    } else if self.mode == ViMode::Normal
                        && modifier == KeyModifiers::NONE
                        && matches!(code, KeyCode::Char('v'))
                    {
                        self.cache.clear();
                        self.mode = ViMode::Visual;
                        ReedlineEvent::Multiple(vec![ReedlineEvent::Esc, ReedlineEvent::Repaint])
                    } else if modifier == KeyModifiers::NONE || modifier == KeyModifiers::SHIFT {
                        // TODO maybe motions, etc. could be modelled with `Sequence` instead
                        self.cache.push(if modifier == KeyModifiers::SHIFT {
                            c.to_ascii_uppercase()
                        } else {
                            c
                        });

                        let res = parse(&mut self.cache.iter().peekable());

                        if !res.is_valid() {
                            self.cache.clear();
                            ReedlineEvent::None
                        } else if res.is_complete(self.mode) {
                            if let Some(mode) = res.changes_mode() {
                                self.mode = mode;
                            }

                            let event = res.to_reedline_event(self);
                            self.cache.clear();
                            event
                        } else {
                            ReedlineEvent::None
                        }
                    } else {
                        ReedlineEvent::None
                    }
                }
                (ViMode::Insert, modifier, KeyCode::Char(c)) => {
                    // Note. The modifier can also be a combination of modifiers, for
                    // example:
                    //     KeyModifiers::CONTROL | KeyModifiers::ALT
                    //     KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT
                    //
                    // Mixed modifiers are used by non american keyboards that have extra
                    // keys like 'alt gr'. Keep this in mind if in the future there are
                    // cases where an event is not being captured
                    self.handle_binding(KeyCombination {
                        modifier: modifiers,
                        key_code: KeyCode::Char(c),
                    })
                    .unwrap_or_else(|| {
                        if modifier == KeyModifiers::NONE
                            || modifier == KeyModifiers::SHIFT
                            || modifier == KeyModifiers::CONTROL | KeyModifiers::ALT
                            || modifier
                                == KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT
                        {
                            ReedlineEvent::Edit(vec![EditCommand::InsertChar(c)])
                        } else {
                            ReedlineEvent::None
                        }
                    })
                }
                (_, KeyModifiers::NONE, KeyCode::Esc) => {
                    self.cache.clear();
                    self.partial_key_sequence = None;
                    self.mode = ViMode::Normal;
                    ReedlineEvent::Multiple(vec![ReedlineEvent::Esc, ReedlineEvent::Repaint])
                }
                (_, KeyModifiers::NONE, KeyCode::Enter) => {
                    self.mode = ViMode::Insert;
                    ReedlineEvent::Enter
                }
                (ViMode::Normal | ViMode::Visual, _, _) => self
                    .handle_binding(KeyCombination {
                        modifier: modifiers,
                        key_code: code,
                    })
                    .unwrap_or(ReedlineEvent::None),
                (ViMode::Insert, _, _) => self
                    .handle_binding(KeyCombination {
                        modifier: modifiers,
                        key_code: code,
                    })
                    .unwrap_or(ReedlineEvent::None),
            },

            Event::Mouse(_) => ReedlineEvent::Mouse,
            Event::Resize(width, height) => ReedlineEvent::Resize(width, height),
            Event::FocusGained => ReedlineEvent::None,
            Event::FocusLost => ReedlineEvent::None,
            Event::Paste(body) => ReedlineEvent::Edit(vec![EditCommand::InsertString(
                body.replace("\r\n", "\n").replace('\r', "\n"),
            )]),
        }
    }

    fn edit_mode(&self) -> PromptEditMode {
        match self.mode {
            ViMode::Normal | ViMode::Visual => PromptEditMode::Vi(PromptViMode::Normal),
            ViMode::Insert => PromptEditMode::Vi(PromptViMode::Insert),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn esc_leads_to_normal_mode_test() {
        let mut vi = Vi::default();
        let esc =
            ReedlineRawEvent::try_from(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
                .unwrap();
        let result = vi.parse_event(esc);

        assert_eq!(
            result,
            ReedlineEvent::Multiple(vec![ReedlineEvent::Esc, ReedlineEvent::Repaint])
        );
        assert!(matches!(vi.mode, ViMode::Normal));
    }

    #[test]
    fn keybinding_without_modifier_test() {
        let mut keybindings = default_vi_normal_keybindings();
        keybindings.add_binding(
            KeyModifiers::NONE,
            KeyCode::Char('e'),
            ReedlineEvent::ClearScreen,
        );

        let mut vi = Vi {
            insert_keybindings: default_vi_insert_keybindings(),
            normal_keybindings: keybindings,
            mode: ViMode::Normal,
            ..Default::default()
        };

        let esc = ReedlineRawEvent::try_from(Event::Key(KeyEvent::new(
            KeyCode::Char('e'),
            KeyModifiers::NONE,
        )))
        .unwrap();
        let result = vi.parse_event(esc);

        assert_eq!(result, ReedlineEvent::ClearScreen);
    }

    #[test]
    fn keybinding_with_shift_modifier_test() {
        let mut keybindings = default_vi_normal_keybindings();
        keybindings.add_binding(
            KeyModifiers::SHIFT,
            KeyCode::Char('$'),
            ReedlineEvent::CtrlD,
        );

        let mut vi = Vi {
            insert_keybindings: default_vi_insert_keybindings(),
            normal_keybindings: keybindings,
            mode: ViMode::Normal,
            ..Default::default()
        };

        let esc = ReedlineRawEvent::try_from(Event::Key(KeyEvent::new(
            KeyCode::Char('$'),
            KeyModifiers::SHIFT,
        )))
        .unwrap();
        let result = vi.parse_event(esc);

        assert_eq!(result, ReedlineEvent::CtrlD);
    }

    #[test]
    fn keybinding_with_super_modifier_test() {
        let mut keybindings = default_vi_normal_keybindings();
        keybindings.add_binding(
            KeyModifiers::SUPER,
            KeyCode::Char('$'),
            ReedlineEvent::CtrlD,
        );

        let mut vi = Vi {
            insert_keybindings: default_vi_insert_keybindings(),
            normal_keybindings: keybindings,
            mode: ViMode::Normal,
            ..Default::default()
        };

        let esc = ReedlineRawEvent::try_from(Event::Key(KeyEvent::new(
            KeyCode::Char('$'),
            KeyModifiers::SUPER,
        )))
        .unwrap();
        let result = vi.parse_event(esc);

        assert_eq!(result, ReedlineEvent::CtrlD);
    }

    #[test]
    fn non_register_modifier_test() {
        let keybindings = default_vi_normal_keybindings();
        let mut vi = Vi {
            insert_keybindings: default_vi_insert_keybindings(),
            normal_keybindings: keybindings,
            mode: ViMode::Normal,
            ..Default::default()
        };

        let esc = ReedlineRawEvent::try_from(Event::Key(KeyEvent::new(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
        )))
        .unwrap();
        let result = vi.parse_event(esc);

        assert_eq!(result, ReedlineEvent::None);
    }
}
