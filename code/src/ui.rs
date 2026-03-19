use iced::{
    Task, Length,
    widget::{
        Container, Row, Theme,
        button, column, container, pane_grid, row, svg, text,
        svg::Handle, pick_list, scrollable, text_input, mouse_area
    },
    padding, font
};

use std::path::PathBuf;
use std::io::Write;

use nix::sys::signal::Signal;

use crate::{
    window::*, data::*, trace::*, style, config, dwarf::*, object
};

const EXTRABOLD: font::Font = font::Font {weight: font::Weight::ExtraBold, ..font::Font::DEFAULT};
const BOLD: font::Font = font::Font {weight: font::Weight::Bold, ..font::Font::DEFAULT};
const SIDERATIO: f32 = 0.25; // (0.1; 0.4)


pub struct Layout {
    status_bar: bool,
    sidebar_left: bool,
    sidebar_right: bool,
    panel: bool,
    panel_mode: config::PanelMode,
    pub panes: pane_grid::State<Pane>,
    _focus: Option<pane_grid::Pane>
}

impl Default for Layout {
    fn default() -> Self {
        let layout = CONFIG.access().as_ref().unwrap().layout.clone().unwrap();
        Layout {
            status_bar: layout.status_bar.unwrap(),
            sidebar_left: layout.sidebar_left.unwrap(),
            sidebar_right: layout.sidebar_right.unwrap(),
            panel: layout.panel.unwrap(),
            panel_mode: layout.panel_mode.unwrap(),
            panes: Self::panes_config(),
            _focus: None
        }
    }
}

impl Layout {
    fn panes_config() -> pane_grid::State<Pane> {
        let bind = CONFIG.access();
        let config = bind.as_ref().unwrap();
        let layout = config.layout.as_ref().unwrap();

        let left = layout.sidebar_left.unwrap();
        let right = layout.sidebar_right.unwrap();
        let panel = layout.panel.unwrap();
        let panel_mode = layout.panel_mode.as_ref().unwrap();

        let (left_ratio, right_ratio) = match panel_mode {
            config::PanelMode::left => {
                if panel {
                    ((SIDERATIO)/(1.0-SIDERATIO), (1.0 - SIDERATIO))
                } else {
                    (SIDERATIO, (1.0 - 2.0*SIDERATIO)/(1.0-SIDERATIO))
                }
            }
            _ => {
                (SIDERATIO, (1.0 - 2.0*SIDERATIO)/(1.0-SIDERATIO))
            }
        };

        let panel_ratio = 1.0 - SIDERATIO;

        let mut list = layout.panes.clone().unwrap();

        list.left.reverse(); //because we are popping them, not iterating through them
        list.right.reverse();
        list.panel.reverse();
        list.main.reverse();

        let panes = SavedState {
            left_sidebar: (Self::serialize(list.left, pane_grid::Axis::Horizontal), left_ratio),
            right_sidebar: (Self::serialize(list.right, pane_grid::Axis::Horizontal), right_ratio),
            panel: (Self::serialize(list.panel, pane_grid::Axis::Vertical), panel_ratio),
            main: Some(Self::serialize(list.main, pane_grid::Axis::Vertical))
        };

        SAVED_STATE.sets(panes.clone());

        let base = Self::base(left, right, panel, &panel_mode, panes);

        pane_grid::State::with_configuration(base)
    }

    fn base(left: bool, right: bool, panel: bool, panel_mode: &config::PanelMode, panes: crate::data::SavedState) -> pane_grid::Configuration<Pane> {
        let left_ratio = panes.left_sidebar.1;
        let right_ratio = panes.right_sidebar.1;

        if panel { return match panel_mode {
            config::PanelMode::full => Self::panel_full(left, right, panes),
            config::PanelMode::middle => Self::panel_middle(left, right, panes),
            config::PanelMode::left => Self::panel_left(left, right, panes),
            config::PanelMode::right => Self::panel_right(left, right, panes)
        };} else { //without the panel
            if left & right {
                return pane_grid::Configuration::Split{
                    axis: pane_grid::Axis::Vertical,
                    ratio: left_ratio,
                    a: Box::new(panes.left_sidebar.0),
                    b: Box::new(pane_grid::Configuration::Split {
                    axis: pane_grid::Axis::Vertical,
                    ratio: right_ratio, // just some math :P, basic percentages
                    a: Box::new(panes.main.unwrap()),
                    b: Box::new(panes.right_sidebar.0)
                })};
            };
            if left {
                return pane_grid::Configuration::Split {
                    axis: pane_grid::Axis::Vertical,
                    ratio: left_ratio,
                    a: Box::new(panes.left_sidebar.0),
                    b: Box::new(panes.main.unwrap())
                };
            };
            if right {
                return pane_grid::Configuration::Split {
                    axis: pane_grid::Axis::Vertical,
                    ratio: right_ratio,
                    a: Box::new(panes.main.unwrap()),
                    b: Box::new(panes.right_sidebar.0)
                };
            }
            return panes.main.unwrap();
        };
    }

    fn panel_full(left: bool, right: bool, panes: crate::data::SavedState) -> pane_grid::Configuration<Pane> {
        let left_ratio = panes.left_sidebar.1;
        let right_ratio = panes.right_sidebar.1;
        let panel_ratio = panes.panel.1;

        if left & right {
            return pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Horizontal,
                ratio: panel_ratio,
                b: Box::new(panes.panel.0),
                a: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: left_ratio,
                a: Box::new(panes.left_sidebar.0),
                b: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: right_ratio, // just some math :P, basic percentages
                a: Box::new(panes.main.unwrap()),
                b: Box::new(panes.right_sidebar.0)
            })})};
        };
        if left {
            return pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Horizontal,
                ratio: panel_ratio,
                b: Box::new(panes.panel.0),
                a: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: left_ratio,
                a: Box::new(panes.left_sidebar.0),
                b: Box::new(panes.main.unwrap())
            })};
        };
        if right {
            return pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Horizontal,
                ratio: panel_ratio,
                b: Box::new(panes.panel.0),
                a: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: right_ratio,
                a: Box::new(panes.main.unwrap()),
                b: Box::new(panes.right_sidebar.0)
            })};
        }
        return pane_grid::Configuration::Split {
            axis: pane_grid::Axis::Horizontal,
            ratio: panel_ratio,
            a: Box::new(panes.main.unwrap()),
            b: Box::new(panes.panel.0)
        };
    }

    fn panel_middle(left: bool, right: bool, panes: crate::data::SavedState) -> pane_grid::Configuration<Pane> {
        let left_ratio = panes.left_sidebar.1;
        let right_ratio = panes.right_sidebar.1;
        let panel_ratio = panes.panel.1;

        if left & right {
            return pane_grid::Configuration::Split{
                axis: pane_grid::Axis::Vertical,
                ratio: left_ratio,
                a: Box::new(panes.left_sidebar.0),
                b: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: right_ratio, // just some math :P, basic percentages
                b: Box::new(panes.right_sidebar.0),
                a: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Horizontal,
                ratio: panel_ratio,
                a: Box::new(panes.main.unwrap()),
                b: Box::new(panes.panel.0)
            })})};
        };
        if left {
            return pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: left_ratio,
                a: Box::new(panes.left_sidebar.0),
                b: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Horizontal,
                ratio: panel_ratio,
                a: Box::new(panes.main.unwrap()),
                b: Box::new(panes.panel.0)
            })};
        };
        if right {
            return pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: right_ratio,
                b: Box::new(panes.right_sidebar.0),
                a: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Horizontal,
                ratio: panel_ratio,
                a: Box::new(panes.main.unwrap()),
                b: Box::new(panes.panel.0)
            })};
        }
        return pane_grid::Configuration::Split {
            axis: pane_grid::Axis::Horizontal,
            ratio: panel_ratio,
            a: Box::new(panes.main.unwrap()),
            b: Box::new(panes.panel.0)
        };
    }

    fn panel_left(left: bool, right: bool, panes: crate::data::SavedState) -> pane_grid::Configuration<Pane> {
        let left_ratio = panes.left_sidebar.1;
        let right_ratio = panes.right_sidebar.1;
        let panel_ratio = panes.panel.1;

        if left & right {
            return pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: right_ratio,
                b: Box::new(panes.right_sidebar.0),
                a: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Horizontal,
                ratio: panel_ratio,
                b: Box::new(panes.panel.0),
                a: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: left_ratio, // just some math :P, basic percentages
                a: Box::new(panes.left_sidebar.0),
                b: Box::new(panes.main.unwrap()),
            })})};
        };
        if left {
            return pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Horizontal,
                ratio: panel_ratio,
                b: Box::new(panes.panel.0),
                a: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: left_ratio,
                a: Box::new(panes.left_sidebar.0),
                b: Box::new(panes.main.unwrap())
            })};
        };
        if right {
            return pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: right_ratio,
                b: Box::new(panes.right_sidebar.0),
                a: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Horizontal,
                ratio: panel_ratio,
                a: Box::new(panes.main.unwrap()),
                b: Box::new(panes.panel.0)
            })};
        }
        return pane_grid::Configuration::Split {
            axis: pane_grid::Axis::Horizontal,
            ratio: panel_ratio,
            a: Box::new(panes.main.unwrap()),
            b: Box::new(panes.panel.0)
        };
    }

    fn panel_right(left: bool, right: bool, panes: crate::data::SavedState) -> pane_grid::Configuration<Pane> {
        let left_ratio = panes.left_sidebar.1;
        let right_ratio = panes.right_sidebar.1;
        let panel_ratio = panes.panel.1;

        if left & right {
            return pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: left_ratio,
                a: Box::new(panes.left_sidebar.0),
                b: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Horizontal,
                ratio: panel_ratio,
                b: Box::new(panes.panel.0),
                a: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: right_ratio, // just some math :P, basic percentages
                a: Box::new(panes.main.unwrap()),
                b: Box::new(panes.right_sidebar.0)
            })})};
        };
        if left {
            return pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: left_ratio,
                a: Box::new(panes.left_sidebar.0),
                b: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Horizontal,
                ratio: panel_ratio,
                a: Box::new(panes.main.unwrap()),
                b: Box::new(panes.panel.0)
            })};
        };
        if right {
            return pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Horizontal,
                ratio: panel_ratio,
                b: Box::new(panes.panel.0),
                a: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: right_ratio,
                a: Box::new(panes.main.unwrap()),
                b: Box::new(panes.right_sidebar.0)
            })};
        }
        return pane_grid::Configuration::Split {
            axis: pane_grid::Axis::Horizontal,
            ratio: panel_ratio,
            a: Box::new(panes.main.unwrap()),
            b: Box::new(panes.panel.0)
        };
    }

    fn serialize(mut list: Vec<config::Pane>, axis: pane_grid::Axis) -> pane_grid::Configuration<Pane> {
        fn magic(current: u8) -> f32 { // now when creating evenly distributed panes, we follow a rule of essentially just making 1/(n+1) series, where n is the amount still left, so yea
            1.0 / (current + 1) as f32
        }

        let pane = match list.pop() {
            None => return pane_grid::Configuration::Pane(Pane::_Empty),
            Some(pane) => match pane {
                config::Pane::assembly => Pane::Assembly(PaneAssembly::default()),
                config::Pane::memory => Pane::Memory(PaneMemory::default()),
                config::Pane::code => Pane::Code(PaneCode::default()),
                config::Pane::registers => Pane::Registers(PaneRegisters::default()),
                config::Pane::stack => Pane::Stack(PaneStack::default()),
                config::Pane::info => Pane::Info,
                config::Pane::control => Pane::Control(PaneControl::default()),
                config::Pane::terminal => Pane::Terminal(PaneTerminal::default())
            }
        };
        if list.is_empty() {
            pane_grid::Configuration::Pane(pane)
        } else {
            pane_grid::Configuration::Split {
                axis,
                ratio: magic(list.len() as u8), // more simple math :P
                a: Box::new(pane_grid::Configuration::Pane(pane)),
                b: Box::new(Self::serialize(list, axis))
            }
        }
    }

    fn get_left_split(&self) -> (pane_grid::Split, f64) { // caller must know that the side bar IS ACTUALLY ACTIVE
        if self.panel { match self.panel_mode {
            config::PanelMode::full => match self.panes.layout() {
                pane_grid::Node::Split { a, ..} => match *a.clone() {
                pane_grid::Node::Split { id, ratio, ..} => (id, ratio as f64),
                _ => panic!()
                },
                _ => panic!()
            },
            config::PanelMode::middle => match self.panes.layout() {
                pane_grid::Node::Split { id, ratio, ..} => (*id, *ratio as f64),
                _ => panic!()
            },
            config::PanelMode::left => match self.panes.layout() {
                pane_grid::Node::Split { a, ..} => match *a.clone() {
                pane_grid::Node::Split { a, ..} => match *a {
                pane_grid::Node::Split { id, ratio, ..} => (id, ratio as f64),
                _ => panic!()
                },
                _ => panic!()
                },
                _ => panic!()
            },
            config::PanelMode::right => match self.panes.layout() {
                pane_grid::Node::Split { id, ratio, ..} => (*id, *ratio as f64),
                _ => panic!()
            },
        }} else {
            match self.panes.layout() {
                pane_grid::Node::Split { id, ratio, ..} => (*id, *ratio as f64),
                _ => panic!()
            }
        }
    }

    fn get_right_split(&self) -> (pane_grid::Split, f64) { // caller must know that BOTH sidebars ARE ACTUALLY ACTIVE
        if self.panel { match self.panel_mode {
            config::PanelMode::full => match self.panes.layout() {
                pane_grid::Node::Split { a, ..} => match *a.clone() {
                pane_grid::Node::Split { b, .. } => match *b {
                pane_grid::Node::Split { id, ratio, ..} => (id, ratio as f64),
                _ => panic!()
                },
                _ => panic!()
                },
                _ => panic!()
            },
            config::PanelMode::middle => match self.panes.layout() {
                pane_grid::Node::Split { b, ..} => match *b.clone() {
                pane_grid::Node::Split { id, ratio, ..} => (id, ratio as f64),
                _ => panic!()
                },
                _ => panic!()
            },
            config::PanelMode::left => match self.panes.layout() {
                pane_grid::Node::Split { id, ratio, ..} => (*id, *ratio as f64),
                _ => panic!()
            },
            config::PanelMode::right => match self.panes.layout() {
                pane_grid::Node::Split { b, ..} => match *b.clone() {
                pane_grid::Node::Split { a, ..} => match *a {
                pane_grid::Node::Split { id, ratio, ..} => (id, ratio as f64),
                _ => panic!()
                },
                _ => panic!()
                },
                _ => panic!()
            }
        }} else {
            match self.panes.layout() {
                pane_grid::Node::Split { b, ..} => match *b.clone() {
                pane_grid::Node::Split { id, ratio, ..} => (id, ratio as f64),
                _ => panic!()
                },
                _ => panic!()
            }
        }
    }

    fn get_nodes(&self) -> (pane_grid::Node, Option<(pane_grid::Node, f32)>, Option<(pane_grid::Node, f32)>, Option<(pane_grid::Node, f32)>) { //main, panel, letf, right
        let mut main = None;
        let mut left = None;
        let mut right = None;
        let mut panel = None;

        let layout = self.panes.layout().clone();

        let random: Option<pane_grid::Split> = match layout.splits().next() {
            Some(split) => Some(split.clone()),
            None => None
        };

        if self.panel { match self.panel_mode {
            config::PanelMode::full => {
                match layout {
                    pane_grid::Node::Pane(_) => panic!(), // there is a panel
                    pane_grid::Node::Split { a, b, ratio, ..} => {
                        panel = Some((*b, ratio));
                        match *a {
                            pane_grid::Node::Pane(pane) => main = Some(pane_grid::Node::Pane(pane)),
                            pane_grid::Node::Split { a, b, ratio, ..} => {
                                if self.sidebar_left {
                                    left = Some((*a, ratio));
                                    match *b {
                                        pane_grid::Node::Pane(pane) => main = Some(pane_grid::Node::Pane(pane)),
                                        pane_grid::Node::Split { a, b, ratio, ..} => {
                                            main = Some(*a);
                                            right = Some((*b, ratio));
                                        }
                                    }
                                } else {
                                    main = Some(*a);
                                    right = Some((*b, ratio));
                                }
                            }
                        }
                    }
                }
            },
            config::PanelMode::middle => {
                match layout {
                    pane_grid::Node::Pane(_) => panic!(),
                    pane_grid::Node::Split { a, b, ratio, ..} => {
                        if self.sidebar_left {
                            left = Some((*a, ratio));
                            match *b {
                                pane_grid::Node::Pane(pane) => main = Some(pane_grid::Node::Pane(pane)),
                                pane_grid::Node::Split { a, b, ratio, ..} => {
                                    if self.sidebar_right {
                                        right = Some((*b, ratio));
                                        match *a {
                                            pane_grid::Node::Pane(_) => panic!(), // there is a panel
                                            pane_grid::Node::Split { a, b, ratio, ..} => {
                                                main = Some(*a);
                                                panel = Some((*b, ratio));
                                            }
                                        }
                                    } else {
                                        main = Some(*a);
                                        panel = Some((*b, ratio));
                                    }
                                }
                            }
                        } else {
                            if self.sidebar_right {
                                right = Some((*b, ratio));
                                match *a {
                                    pane_grid::Node::Pane(_) => panic!(),
                                    pane_grid::Node::Split { a, b, ratio, ..} => {
                                        main = Some(*a);
                                        panel = Some((*b, ratio));
                                    }
                                }
                            } else {
                                main = Some(*a);
                                panel = Some((*b, ratio));
                            }
                        }
                    }
                }
            },
            config::PanelMode::left => {
                match layout {
                    pane_grid::Node::Pane(_) => panic!(),
                    pane_grid::Node::Split { a, b, ratio, ..} => {
                        if self.sidebar_right {
                            right = Some((*b, ratio));
                            match *a {
                                pane_grid::Node::Pane(_) => panic!(),
                                pane_grid::Node::Split { a, b, ratio, ..} => {
                                    if self.sidebar_left {
                                        panel = Some((*b, ratio));
                                        match *a {
                                            pane_grid::Node::Pane(_) => panic!(),
                                            pane_grid::Node::Split { a, b, ratio, .. } => {
                                                left = Some((*a, ratio));
                                                main = Some(*b);
                                            }
                                        }
                                    } else {
                                        main = Some(*a);
                                        panel = Some((*b, ratio));
                                    }
                                }
                            }
                        } else {
                            if self.sidebar_left {
                                panel = Some((*b, ratio));
                                match *a {
                                    pane_grid::Node::Pane(_) => panic!(),
                                    pane_grid::Node::Split { a, b, ratio, ..} => {
                                        left = Some((*a, ratio));
                                        main = Some(*b)
                                    }
                                }
                            } else {
                                main = Some(*a);
                                panel = Some((*b, ratio));
                            }
                        }
                    }
                }
            },
            config::PanelMode::right => {
                match layout {
                    pane_grid::Node::Pane(_) => panic!(),
                    pane_grid::Node::Split { a, b, ratio, ..} => {
                        if self.sidebar_left {
                            left = Some((*a, ratio));
                            match *b {
                                pane_grid::Node::Pane(_) => panic!(),
                                pane_grid::Node::Split { a, b, ratio, ..} => {
                                    if self.sidebar_right {
                                        panel = Some((*b, ratio));
                                        match *a {
                                            pane_grid::Node::Pane(_) => panic!(),
                                            pane_grid::Node::Split { a, b, ratio, ..} => {
                                                main = Some(*a);
                                                right = Some((*b, ratio));
                                            }
                                        }
                                    } else {
                                        main = Some(*a);
                                        panel = Some((*b, ratio));
                                    }
                                }
                            }
                        } else {
                            if self.sidebar_right {
                                panel = Some((*b, ratio));
                                match *a {
                                    pane_grid::Node::Pane(_) => panic!(),
                                    pane_grid::Node::Split { a, b, ratio, ..} => {
                                        main = Some(*a);
                                        right = Some((*b, ratio));
                                    }
                                }
                            } else {
                                main = Some(*a);
                                panel = Some((*b, ratio));
                            }
                        }
                    }
                }
            }
        }} else {
            match layout {
                pane_grid::Node::Pane(pane) => main = Some(pane_grid::Node::Pane(pane)),
                pane_grid::Node::Split { a, b, ratio, ..} => {
                    if self.sidebar_left {
                        left = Some((*a, ratio));
                        match *b {
                            pane_grid::Node::Pane(pane) => main = Some(pane_grid::Node::Pane(pane)),
                            pane_grid::Node::Split { a, b, ratio, ..} => {
                                main = Some(*a);
                                right = Some((*b, ratio));
                            }
                        }
                    } else {
                        main = Some(*a);
                        right = Some((*b, ratio));
                    }
                }
            }
        };
        if right.is_some() & !self.sidebar_right {
            main = Some(pane_grid::Node::Split {
                id: random.unwrap(), // a random Split, we just need to construct the Node
                axis: pane_grid::Axis::Vertical,
                ratio: right.as_ref().unwrap().1,
                a: Box::new(main.unwrap()),
                b: Box::new(right.unwrap().0)
            });
            return (main.unwrap(), left, None, panel);
        }

        (main.unwrap(), left, right, panel)
    }

    fn node_to_configuration(&self, node: &pane_grid::Node) -> pane_grid::Configuration<Pane> { // a recursive function, as this is the easiest way, also, its only for the row structures so its actually pretty easy
        match node {
            pane_grid::Node::Pane(pane) => pane_grid::Configuration::Pane(self.panes.get(*pane).unwrap().clone()), //retrive the state of the pane
            pane_grid::Node::Split { id, axis, ratio, a, b } => {
                pane_grid::Configuration::Split {
                    axis: *axis,
                    ratio: *ratio,
                    a: Box::new(self.node_to_configuration(a)),
                    b: Box::new(self.node_to_configuration(b))
                }
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum Base {
    #[default]
    Hex,
    Dec,
    Oct,
    Bin,
}

impl Base {
    pub fn form(&self, num: u64) -> String {
        match self {
            Self::Hex => format!("0x{:x}", num),
            Self::Dec => format!("{}", num),
            Self::Oct => format!("0o{:o}", num),
            Self::Bin => format!("0b{:b}", num),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum ByteBase {
    #[default]
    Hex,
    Dec,
    Chr
}

impl ByteBase {
    fn form(&self, num: u8) -> String {
        match self {
            Self::Hex => format!("{:02x}", num),
            Self::Dec => format!("{}", num),
            Self::Chr => format!("{}", if num.is_ascii_graphic() {(num as char).to_string()} else {if num == 0x20 {"' '"} else {"'.'"}.to_string()}),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Pane { // Generic enum for all bars (completed widgets that can be moved around inside a window) (they will have their own structs if they need)
    Memory(PaneMemory),
    Stack(PaneStack),
    Code(PaneCode),
    Assembly(PaneAssembly),
    Registers(PaneRegisters),
    Info, // ELF dump
    Control(PaneControl),
    Terminal(PaneTerminal),
    _Empty //just in case
}

impl Pane {
    fn memory(&mut self) -> &mut PaneMemory {
        match self {
            Pane::Memory(inner) => inner,
            _ => panic!()
        }
    }
    fn code(&mut self) -> &mut PaneCode {
        match self {
            Pane::Code(inner) => inner,
            _ => panic!()
        }
    }
    fn registers(&mut self) -> &mut PaneRegisters {
        match self {
            Pane::Registers(inner) => inner,
            _ => panic!()
        }
    }
    fn control(&mut self) -> &mut PaneControl {
        match self {
            Pane::Control(inner) => inner,
            _ => panic!()
        }
    }
    fn terminal(&mut self) -> &mut PaneTerminal {
        match self {
            Pane::Terminal(inner) => inner,
            _ => panic!()
        }
    }
    fn stack(&mut self) -> &mut PaneStack {
        match self {
            Pane::Stack(inner) => inner,
            _ => panic!()
        }
    }
    fn assembly(&mut self) -> &mut PaneAssembly {
        match self {
            Pane::Assembly(inner) => inner,
            _ => panic!()
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PaneMemory {
    pub field: String,
    incorrect: bool,
    pub address: u64, // where are we in memory, we read extra 2KB around this area and store to global data, and update only when we get outside of this region, for read effectivity
    read_address: u64, // where are we in memory, we read extra 2KB around this area and store to global data, and update only when we get outside of this region, for read effectivity
    data: Vec<u8>,
    more_bytes: bool, // 4 or 8
    format: ByteBase,
    read_error: bool // if read error occurs, show a button to take the user back (resets the address to a correct map)
}

#[derive(Debug, Clone, Default)]
struct PaneStack {
    open: Vec<bool>,
    unique: u32 // id of the last update (in order to reload the open vec)
}

#[derive(Debug, Clone)]
pub struct PaneCode {
    pub dir: Option<String>,
    pub file: Option<String>,
    update: bool,
    breakpoints: Vec<Option<u64>>,
    scrollable: scrollable::Id,
    viewport: Option<scrollable::Viewport>
}

impl Default for PaneCode {
    fn default() -> Self {
        Self {
            update: true,
            dir: None,
            file: None,
            breakpoints: Vec::new(),
            scrollable: scrollable::Id::unique(),
            viewport: None
        }
    }
}

#[derive(Debug, Clone)]
struct PaneAssembly {
    scrollable: scrollable::Id
}

impl Default for PaneAssembly {
    fn default() -> Self {
        Self { scrollable: scrollable::Id::unique() }
    }
}

#[derive(Debug, Clone, Default)]
struct PaneRegisters {
    format: Base
}

#[derive(Debug, Clone, Default)]
struct PaneControl {
    selected_signal: Option<Signal>,
}

#[derive(Debug, Clone, Default)]
struct PaneTerminal {
    input: String
}

#[derive(Debug, Clone)]
pub enum LayoutMessage {
    SidebarLeftToggle,
    SidebarRightToggle,
    PanelToggle,
    _Focus(pane_grid::Pane),
    Drag(pane_grid::DragEvent),
    Resize(pane_grid::ResizeEvent),
}

#[derive(Debug, Clone)]
pub enum PaneMessage {
    // Control
    ControlSelectSignal(pane_grid::Pane, Signal),
    // Registers
    RegistersChangeFormat(pane_grid::Pane, Base),
    // Code
    CodeSelectDir(pane_grid::Pane, String),
    CodeSelectFile(pane_grid::Pane, String),
    CodeLoad(Option<pane_grid::Pane>, SourceIndex, String),
    CodeBreakpoints(pane_grid::Pane, Vec<Option<u64>>),
    CodeToggleUpdate(pane_grid::Pane),
    CodeScroll(pane_grid::Pane, scrollable::Viewport),
    // Memory
    MemoryChangeFormat(pane_grid::Pane, ByteBase),
    MemoryToggleSize(pane_grid::Pane),
    MemoryInput(pane_grid::Pane, String),
    MemorySubmit(pane_grid::Pane),
    MemoryPaste(pane_grid::Pane, String),
    MemoryAddress(pane_grid::Pane, iced::mouse::ScrollDelta, i8), //the i8 is as a signed multiplier, mirroring the axis
    MemoryReset(pane_grid::Pane),
    // Terminal
    TerminalType(pane_grid::Pane, String),
    TerminalPaste(pane_grid::Pane, String),
    TerminalSend(pane_grid::Pane),
    // Assembly
    AssemblyUpdate(Result<(crate::dwarf::Assembly, usize), ()>),
    //Stack
    StackUpdate(pane_grid::Pane),
    StackCollapse(pane_grid::Pane, usize),
    StackExpand(pane_grid::Pane, usize)
}

pub fn content(state: &State) -> Container<'_, Message> {
    container(column(
        if state.layout.status_bar {vec![
            toolbar(state, 45).into(),
            main_frame(state).into(),
            statusbar(state, 20).into()
        ]} else {vec![
            toolbar(state, 45).into(),
            main_frame(state).into(),
        ]}
    ))
}

fn toolbar<'a>(state: &State, height: usize) -> Container<'a, Message> {

    fn buttons<'a>(state: &State, size: u16) -> [button::Button<'a, Message>; 4] {
        let load_file = svg_button("icons/load_file.svg", size, None).on_press(Message::Operation(Operation::LoadFile)).style(style::bar_button);

        // Toggle buttons:
        let sidebar_left = svg_button("icons/sidebar_left.svg", size,
        Some(if state.layout.sidebar_left {style::bar_svg_toggled} else {style::bar_svg})
        ).style(if state.layout.sidebar_left {style::bar_button_toggled} else {style::bar_button})
        .on_press(Message::Layout(LayoutMessage::SidebarLeftToggle));

        let sidebar_right = svg_button("icons/sidebar_right.svg", size,
        Some(if state.layout.sidebar_right {style::bar_svg_toggled} else {style::bar_svg})
        ).style(if state.layout.sidebar_right {style::bar_button_toggled} else {style::bar_button})
        .on_press(Message::Layout(LayoutMessage::SidebarRightToggle));

        let panel = svg_button("icons/panel.svg", size,
        Some(if state.layout.panel {style::bar_svg_toggled} else {style::bar_svg})
        ).style(if state.layout.panel {style::bar_button_toggled} else {style::bar_button})
        .on_press(Message::Layout(LayoutMessage::PanelToggle));

        [load_file, sidebar_left, sidebar_right, panel] //extend if needed
    }

    let padding = 5.;

    let (
        load_file,
        sidebar_left,
        sidebar_right,
        panel
    ) = buttons(state, ((height as f32)-padding*2.) as u16).into();

    let left_buttons: Row<'_, Message> = row![
        load_file
    ].spacing(padding);

    let right_buttons: Row<'_, Message> = row![
        sidebar_left,
        sidebar_right,
        panel
    ].spacing(padding);

    container(row([
        left_buttons.into(),
        widget_fill().into(),
        right_buttons.into()
    ])).height(Length::Fixed(height as f32))
    .width(Length::Fill)
    .padding(padding) //padding around the buttons
    .style(style::bar)
}

fn statusbar<'a>(state: &State, height: u16) -> Container<'a, Message> {
    fn status_text<'a>(string: String, mut content: Row<'a, Message>, size: u16, style: Option<impl Fn(&Theme) -> text::Style + 'a>) -> Row<'a, Message> {
        content = if style.is_some() {
            content.push(
                text(string).size(size).center().style(style.unwrap())
            )
        } else {
            content.push(
                text(string).size(size).center()
            )
        };
        content
    }

    let default: Option<fn(&Theme) -> text::Style> = None;

    let size = height - 7;
    let mut content: Row<'_, Message> = row![];

    content = match FILE.access().as_ref() {
        None => status_text("File not loaded.".to_string(), content, size, default),
        Some(file) => status_text(format!("File: {}", file.file_name().unwrap().to_str().unwrap()), content, size, Some(style::widget_text)),
    };
    content = content.push(delimiter(10));

    content = match PID.access().as_ref() {
        None => status_text("Program not running".to_string(), content, size, default),
        Some(pid) => status_text(format!("Pid: {}", pid), content, size, Some(style::widget_text)),
    };

    if PID.access().is_some() {
        content = content.push(delimiter(10));

        if state.internal.stopped {
            let mut msg = match state.status.unwrap() {
                nix::sys::wait::WaitStatus::Signaled(_, signal, core_dump) => format!("Stopped: {signal}"),
                _ => "Stopped".to_string()
            };

            if state.internal.breakpoint {
                msg = "Breakpoint".to_string()
            }

            if state.internal.manual {
                msg = "Stopped".to_string()
            }
            content = status_text(msg, content, size, default);

            match &state.internal.pane.file {
                Some(index) => {
                    let bind = SOURCE.access();
                    let source = bind.as_ref().unwrap();
                    let file = source.index_with_line(index).path.to_str().unwrap();
                    let msg = format!("At line: {} in file {}", index.line, file);

                    content = content.push(widget_fill());
                    content = status_text(msg, content, size, default);
                }
                None => ()
            };
        } else {
            content = status_text("Running...".to_string(), content, size, default);
        };
    }

    match state.status {
        Some(nix::sys::wait::WaitStatus::Exited(pid, ecode)) => {
            content = content.push(delimiter(10));
            let msg = format!("Program with pid {pid} exited: {ecode:-}");
            content = status_text(msg, content, size, Some(style::error));
        },
        _ => ()
    };

    container(
        content
    ).height(Length::Fixed(height as f32))
    .width(Length::Fill)
    .padding(3)
    .style(style::bar)
}



fn main_frame<'a>(state: &'a State) -> Container<'a, Message> {
    container(
        pane_grid(&state.layout.panes, |id, pane, _maximized| pane_view(id, pane, state)).spacing(10)
        .width(Length::Fill)
        .height(Length::Fill)
        .on_click(|pane| Message::Layout(LayoutMessage::_Focus(pane)))
        .on_drag(|drag_event| Message::Layout(LayoutMessage::Drag(drag_event)))
        .on_resize(10, |resize_event| Message::Layout(LayoutMessage::Resize(resize_event)))
        .spacing(2)
    ).center(Length::Fill)
    .width(Length::Fill)
    .height(Length::Fill)
}

fn pane_view<'a>(id: pane_grid::Pane, pane: &'a Pane, state: &'a State) -> pane_grid::Content<'a, Message> {
    let (content, titlebar) = match pane {
        Pane::Code(pane) => (pane_view_code(pane, state, id), pane_titlebar("Code", "icons/pane_source.svg")),
        Pane::Control(pane) => (pane_view_control(pane, state, id), pane_titlebar("Control", "icons/pane_control.svg")),
        Pane::Memory(pane) => (pane_view_memory(pane, id), pane_titlebar("Memory", "icons/pane_memory.svg")),
        Pane::Stack(pane) => (pane_view_stack(pane, state, id), pane_titlebar("CallStack", "icons/pane_stack.svg")),
        Pane::Registers(pane) => (pane_view_registers(pane, id), pane_titlebar("Registers", "icons/pane_registers.svg")),
        Pane::Assembly(pane) => (pane_view_assembly(pane, state, id), pane_titlebar("Assembly", "icons/pane_assembly.svg")),
        Pane::Terminal(pane) => (pane_view_terminal(pane, state, id), pane_titlebar("Terminal", "icons/pane_terminal.svg")),
        Pane::Info => (pane_view_info(), pane_titlebar("ELF Info", "icons/pane_info.svg")),

        _ => (container(text("Some other pane")), pane_grid::TitleBar::new(text("UNDEFINED")))
    };

    pane_grid::Content::new(content).title_bar(titlebar)
}

fn pane_titlebar<'a>(title: &'a str, icon: &'a str) -> pane_grid::TitleBar<'a, Message> {
    let height = 25;
    pane_grid::TitleBar::new(
        row![
            svg(Handle::from_memory(Asset::get(icon).unwrap().data))
            .height(Length::Fill)
            .width(Length::Shrink)
            .style(style::bar_svg),
            text(title)
            .size(15)
            .height(Length::Fill)
            .width(Length::Shrink)
            .center()
        ].spacing(5).padding(padding::left(3)).height(height)
    ).style(style::pane_title)
}


fn pane_view_code<'a>(pane: &'a PaneCode, state: &'a State, id: pane_grid::Pane) -> Container<'a, Message> {
    if state.internal.no_debug {
        return program_message("No debugging informatio")
    }

    let size = 30;

    let update_button = svg_button(
        "icons/view.svg",
        size,
        Some(if pane.update {style::widget_svg_toggled} else {style::widget_svg}))
    .style(if pane.update {style::widget_button_toggled} else {style::widget_button})
    .on_press(Message::Pane(PaneMessage::CodeToggleUpdate(id)));

    let bind = SOURCE.access();
    if bind.is_none() {
        if state.internal.no_debug {
            return program_message("No debugging information present in the file.");
        }
        return program_message("Load the program to display source code.");
    };

    let source = bind.clone().unwrap();

    let mut dirs: Vec<String> = source.keys().map(|path| String::from(path.to_str().unwrap())).collect();
    dirs.sort();
    dirs.dedup();

    let hash_list: pick_list::PickList<'_, String, Vec<String>, String, Message> = pick_list(dirs.clone(), pane.dir.clone(), move |path| Message::Pane(PaneMessage::CodeSelectDir(id, path)));

    let mut files: Vec<String> = source[&PathBuf::from(pane.dir.clone().unwrap_or(dirs[0].clone()))].iter().map(|file| String::from(file.path.to_str().unwrap())).collect();
    files.sort();
    files.dedup();

    let file_list: pick_list::PickList<'_, String, Vec<String>, String, Message> = pick_list(files, pane.file.clone(), move |path| Message::Pane(PaneMessage::CodeSelectFile(id, path)));

    let code = if pane.dir.is_some() {match &pane.file {
        Some(file) => {
            let comp_path = PathBuf::from(pane.dir.as_ref().unwrap());
            let file_path = PathBuf::from(file);
            let code = code_display(comp_path, file_path, source, pane, &state.internal.pane.file);
            if code.is_ok() {
                container(scrollable(
                code.unwrap()
                ).direction(scrollable::Direction::Both { vertical: scrollable::Scrollbar::new(), horizontal: scrollable::Scrollbar::new() })
                .height(Length::Fill)
                .width(Length::Fill)
                .id(pane.scrollable.clone())
                .on_scroll(move |view| Message::Pane(PaneMessage::CodeScroll(id, view))))
            } else {
                program_message("File contents not loaded.")
            }
        },
        None => program_message("File not selected.").into()
    }} else {
        program_message("Directory not selected.").into()
    };

    container(
        column![
            row![hash_list, file_list, widget_fill(), update_button].spacing(10).padding(3).height(size+6),
            code
        ]
    ).style(style::back)
}

fn code_display<'a>(comp_path: PathBuf, file_path: PathBuf, source: SourceMap, pane: &'a PaneCode, line: &Option<SourceIndex>) -> Result<Row<'a, Message>, ()> {

    let (file, index) = match source.get_file(comp_path.clone(), file_path.clone()) {
        Some(file) => file,
        None => return Err(())
    };
    if file.content.is_none() {
        return Err(())
    }

    let size = 25;

    let mut lines = Vec::new();
    let breakpoints = column(
        pane.breakpoints.iter().enumerate().map(|(index, address)| {
            lines.push(index);
            breakpoint_button(*address, size).into()
        })
    );

    let highlight = match line {
        Some(index) => {
            let real_name = &source.index_with_line(index).path;
            if index.hash_path == comp_path && file_path == *real_name {
                index.line
            } else {
                0
            }
        },
        None => 0
    };

    let line_number = column(
        lines.iter().map(|num|
            if highlight as usize == *num+1  {
                text(num + 1).style(style::line).font(BOLD)
            } else {
                text(num + 1).style(style::weak)
            }.size(size-8).height(size).center().into()
        )
    ).width(Length::Shrink).align_x(iced::Right);

    let text = text(file.content.clone().unwrap()).size(size-8)
    .line_height(text::LineHeight::Absolute(iced::Pixels(size as f32)))
    .wrapping(text::Wrapping::None);

    Ok(row![
        breakpoints, line_number, container("").width(5), text
    ].spacing(0)
    .padding(5))
}

fn breakpoint_button<'a>(address: Option<u64>, size: u16) -> button::Button<'a, Message> {
    match address {
        Some(address) => {
            let present = BREAKPOINTS.access().as_ref().unwrap().contains_key(&address);
            button(
                svg(Handle::from_memory(Asset::get("icons/signal.svg").unwrap().data))
                .style(if present {style::breakpoint_svg_toggled} else {style::breakpoint_svg})
                .width(Length::Fill)
                .height(Length::Fill)
            ).style(style::breakpoint)
            .on_press(if present {Message::Operation(Operation::BreakpointRemove(address))} else {Message::Operation(Operation::BreakpointAdd(address))})
            .width(size)
            .height(size)
            .padding(7)
        },
        None => button("")
            .style(style::breakpoint)
            .width(size)
            .height(size)
    }
}

pub fn code_panes_update(state: &mut State) -> Option<(Task<Message>, Task<Message>)> {
    let file = match &state.internal.pane.file {
        Some(file) => file,
        None => return None
    };

    let mut scroll_tasks = Vec::new();
    let mut load_tasks = Vec::new();

    let panes = &mut state.layout.panes;
    for (id, pane) in panes.iter_mut() {
        let data = match pane {
            Pane::Code(inner) => inner,
            _ => continue
        };
        if !data.update {
            continue;
        }
        let (scroll, load) = code_update(*id, file, data);

        scroll_tasks.push(scroll);
        load_tasks.push(load);
    }

    Some((
        Task::batch(scroll_tasks),
        Task::batch(load_tasks)
    ))
}

fn code_update(id: pane_grid::Pane, file: &SourceIndex, pane: &mut PaneCode) -> (Task<Message>, Task<Message>) {
    let size = 25;
    let new_dir = Some(file.hash_path.to_str().unwrap().to_string());
    let file_name = SOURCE.access().as_ref().unwrap().index_with_line(file).path.clone().to_str().unwrap().to_string();

    let offset = ((file.line as i32 - 3) * size).max(0);
    let scroll = scrollable::AbsoluteOffset {x: 0., y: offset as f32};


    if new_dir == pane.dir && Some(file_name.clone()) == pane.file {
        let view = match pane.viewport {
            Some(view) => view,
            None => return (scrollable::scroll_to(pane.scrollable.clone(), scroll), Task::none())
        };
        let start = view.absolute_offset().y;
        let end = (view.bounds().height - 6.*size as f32).max(0.);
        let range = start..start+end;

        if range.contains(&(offset as f32)) {
            return (Task::none(), Task::none());
        };
        if offset as f32 > range.end {
            let scroll = scrollable::AbsoluteOffset { x: 0., y: offset as f32 - end};
            (scrollable::scroll_to(pane.scrollable.clone(), scroll), Task::none())
        } else {
            (scrollable::scroll_to(pane.scrollable.clone(), scroll), Task::none())
        }
    } else {
        pane.dir = new_dir;
        (scrollable::scroll_to(pane.scrollable.clone(), scroll), Task::done(Message::Pane(PaneMessage::CodeSelectFile(id, file_name))))
    }
}

pub fn check_for_code(state: &mut State) -> bool {
    for (_, pane) in state.layout.panes.iter() {
        match pane {
            Pane::Code(_) => return true,
            _ => ()
        }
    }
    false
}


fn pane_view_control<'a>(pane: &'a PaneControl, state: &'a State, id: pane_grid::Pane) -> Container<'a, Message> {
    let size = 30;

    let file = FILE.access().is_some();
    let run = PID.access().is_some();
    let stopped = state.internal.stopped;

    let start_stop = if run {
        svg_button("icons/stop.svg", size, Some(style::widget_svg))
        .style(style::widget_button)
        .on_press(Message::Operation(Operation::StopTracee))
    } else {
        svg_button("icons/run.svg", size, Some(if file {style::widget_svg} else {style::button_svg_disabled}))
        .style(style::widget_button)
        .on_press_maybe(if file {Some(Message::Operation(Operation::RunTracee))} else {None})
    };

    let pause_cont = if stopped {
        svg_button("icons/continue.svg", size, Some(if run {style::widget_svg} else {style::button_svg_disabled}))
        .on_press_maybe(if run {Some(Message::Operation(Operation::Continue))} else {None})
        .style(style::widget_button)
    } else {
        svg_button("icons/pause.svg", size, Some(if run {style::widget_svg} else {style::button_svg_disabled}))
        .on_press_maybe(if run {Some(Message::Operation(Operation::Pause))} else {None})
        .style(style::widget_button)
    };

    let step = svg_button("icons/step.svg", size, Some(if stopped {style::widget_svg} else {style::button_svg_disabled}))
    .on_press_maybe(if stopped {Some(Message::Operation(Operation::Step))} else {None})
    .style(style::widget_button);

    let source_step = svg_button(
        "icons/source_step.svg",
        size, 
        Some(
            if stopped & !state.internal.no_debug & state.last_signal.is_none() {style::widget_svg}
            else {style::button_svg_disabled}
        )
    ).on_press_maybe(
        if stopped & !state.internal.no_debug & state.last_signal.is_none() {
            Some(Message::Operation(Operation::SourceStep))
        } else {None}
    ).style(style::widget_button);

    let kill = svg_button("icons/signal_kill.svg", size, Some(if run {style::widget_svg} else {style::button_svg_disabled}))
    .on_press_maybe(if run {Some(Message::Operation(Operation::Kill))} else {None})
    .style(style::widget_button);

    let signal = svg_button("icons/signal.svg", size, Some(if pane.selected_signal.is_some() {style::widget_svg} else {style::button_svg_disabled}))
    .on_press_maybe(if pane.selected_signal.is_some() {Some(Message::Operation(Operation::Signal(pane.selected_signal.unwrap())))} else {None})
    .style(style::widget_button);

    let signals = [
        Signal::SIGKILL,
        Signal::SIGINT,
        Signal::SIGQUIT,
        Signal::SIGHUP,
        Signal::SIGTRAP,
        Signal::SIGCONT,
        Signal::SIGABRT,
        Signal::SIGFPE,
        Signal::SIGUSR1,
        Signal::SIGUSR2,
        Signal::SIGSEGV,
        Signal::SIGTERM,
        Signal::SIGCHLD,
        Signal::SIGSTOP,
        Signal::SIGTSTP,
    ];

    let select = pick_list(signals, pane.selected_signal, move |signal| Message::Pane(PaneMessage::ControlSelectSignal(id, signal)))
    .placeholder("Signal...");

    let content = container(row![
        start_stop,
        pause_cont,
        step,
        source_step,
        kill,
        signal,
        select
    ].padding(3)).style(style::back).width(Length::Fill);
    content
}

fn pane_view_memory<'a>(pane: &'a PaneMemory, id: pane_grid::Pane) -> Container<'a, Message> {
    if MEMORY.access().is_none() {
        return program_message("Start the program to display memory.");
    };

    let size = 30;

    let button_hex: button::Button<'_, Message> = button(
        text("0x").center().font(EXTRABOLD).size(size - 12)
        .style(if pane.format == ByteBase::Hex {style::widget_text_toggled} else {style::widget_text})
    ).padding(0)
    .height(size)
    .width(size)
    .style(if pane.format == ByteBase::Hex {style::widget_button_toggled} else {style::widget_button})
    .on_press(Message::Pane(PaneMessage::MemoryChangeFormat(id, ByteBase::Hex)));

    let button_dec: button::Button<'_, Message> = button(
        text("10").center().font(EXTRABOLD).size(size - 12)
        .style(if pane.format == ByteBase::Dec {style::widget_text_toggled} else {style::widget_text})
    ).padding(0)
    .height(size)
    .width(size)
    .style(if pane.format == ByteBase::Dec {style::widget_button_toggled} else {style::widget_button})
    .on_press(Message::Pane(PaneMessage::MemoryChangeFormat(id, ByteBase::Dec)));

    let button_chr: button::Button<'_, Message> = button(
        text("A").center().font(EXTRABOLD).size(size - 12)
        .style(if pane.format == ByteBase::Chr {style::widget_text_toggled} else {style::widget_text})
    ).padding(0)
    .height(size)
    .width(size)
    .style(if pane.format == ByteBase::Chr {style::widget_button_toggled} else {style::widget_button})
    .on_press(Message::Pane(PaneMessage::MemoryChangeFormat(id, ByteBase::Chr)));

    let bytesize: button::Button<'_, Message> = button(
        text(if pane.more_bytes {"8"} else {"4"}).center().size(size - 12).style(style::widget_text)
    )
    .padding(0)
    .height(size)
    .width(size)
    .style(style::widget_button)
    .on_press(Message::Pane(PaneMessage::MemoryToggleSize(id)));

    let address: text_input::TextInput<'_, Message> = text_input("0x...", &pane.field)
    .on_input(move |data| Message::Pane(PaneMessage::MemoryInput(id, data)))
    .on_submit(Message::Pane(PaneMessage::MemorySubmit(id)))
    .on_paste(move |data| Message::Pane(PaneMessage::MemoryPaste(id, data)))
    .size(size - 12)
    .line_height(iced::Pixels(size as f32 - 10.))
    .width(Length::Fill)
    .style(|theme, status| style::address(theme, status, pane.incorrect));

    let field = row![
        text("Address:").size(size - 12).center().height(size),
        container(mouse_area(address).on_scroll(move |delta| Message::Pane(PaneMessage::MemoryAddress(id, delta, 1)))).width(Length::FillPortion(4)),
        widget_fill(),
        button_hex,
        button_dec,
        button_chr,
        bytesize
    ].spacing(2).padding(3).height(Length::Shrink);

    let test = match test_memory(pane.address) {
        Err(test) => test,
        Ok(()) => false
    };

    let memory = if test {
        container(column![
            text("Read out of memory map bounds.").width(Length::Fill).center().style(style::error),
            container(button(text("Reload to begining of program map")).on_press(Message::Pane(PaneMessage::MemoryReset(id)))).width(Length::Fill).center_x(Length::Fill)
        ]).center(Length::Fill).width(Length::Fill).height(Length::Fill)
    } else {
        let data: &Vec<u8>= &pane.data;

        let mut addresses: Vec<u64> = Vec::new();
        let mut bytes: Vec<Vec<u8>> = if pane.more_bytes {
            vec![Vec::new(); 8]
        } else {
            vec![Vec::new(); 4]
        };

        let len = bytes.len();
        let mut pointer = pane.address - pane.address % len as u64;
        let start = pointer - pane.read_address;

        for (i, byte) in data[start as usize..].iter().enumerate() {
            if i % len == 0 {
                addresses.push(pointer);
                pointer += len as u64;
            };

            bytes[i % len].push(*byte);
            if i == 40*len - 1 { // 40 lines
                break;
            }
        };

        let size = 25;

        let address_column: iced::widget::Column<'_, Message> = column(
            addresses.iter().map(|addr| text(
                format!("0x{:06x}", addr)
            ).size(size - 5)
            .height(size)
            .center()
            .style(style::weak)
            .into())
        );

        let byte_columns: iced::widget::Row<'_, Message> = row(bytes.iter().map(|col| column(
            col.iter().map(|byte| text(
                pane.format.form(*byte)
            ).size(size - 5)
            .height(size)
            .center()
            .style(style::widget_text)
            .into())
        ).align_x(iced::Alignment::Center)
        .width(match pane.format {
            ByteBase::Chr => Length::Fixed(20 as f32),
            _ => Length::Shrink
        })
        .into())
        ).spacing(match pane.format {
            ByteBase::Hex => 10,
            ByteBase::Chr => 10,
            ByteBase::Dec => 15
        });

        container(mouse_area(
            row![address_column, byte_columns]
            .height(Length::Fill)
            .width(Length::Fill)
            .padding(5)
            .spacing(20)
        ).on_scroll(move |delta| Message::Pane(PaneMessage::MemoryAddress(id, delta, -3))))
    };

    let content = container(column![
        field,
        memory
    ]).style(style::back);
    content
}


fn pane_view_stack<'a>(pane: &'a PaneStack, state: &'a State, id: pane_grid::Pane) -> Container<'a, Message> {
    let stack = match &state.internal.pane.stack {
        Some(stack) => stack,
        None => if PID.access().is_some() {
            return program_message("Stack data not loaded")
        } else {
            return program_message("Start the program to display stack data.")
        }
    };

    if pane.unique != state.internal.pane.unique_stack {
        return container(column![
            text("Old Stack Data").width(Length::Fill).center(),
            container(button(text("Update Stack")).on_press(Message::Pane(PaneMessage::StackUpdate(id)))).width(Length::Fill).center_x(Length::Fill)
        ]).center(Length::Fill).width(Length::Fill).height(Length::Fill)
    };

    let size: u16 = 23;

    let open_vec = &pane.open;

    let mut collapse = column![].width(size);
    let mut lines = column![];

    for (i, open) in open_vec.iter().enumerate() {
        if !open {continue;}
        let (depth, line) = &stack[i];
        let data = if *depth == 0 {
            text(line).style(style::widget_text)
        } else {
            text(line)
        }.height(size).size(size-5);
        lines = lines.push(
            container(data)
            .padding(padding::left(size*depth.checked_sub(1).unwrap_or(0) as u16)) // removing the indent on the closing brackets of params, while keeping correct collapse rules
        );
        match stack.get(i+1) {
            Some((next_depth, _)) => if next_depth > depth {
                collapse = collapse.push(collapse_button(open_vec[i+1], i, size, id));
            } else {
                collapse = collapse.push(container("").height(size));
            }
            None => ()
        };
    }

    let content = container(
        scrollable(
            row![collapse, lines].padding(padding::Padding {bottom: 10., right: 10., ..Default::default()}) // padding for the scrollbars
        ).direction(scrollable::Direction::Both { vertical: scrollable::Scrollbar::new(), horizontal: scrollable::Scrollbar::new() })
        .width(Length::Fill)
        .height(Length::Fill)
    ).style(style::back);
    content
}

fn collapse_button<'a>(open: bool, index: usize, size: u16, id: pane_grid::Pane) -> button::Button<'a, Message> {
    if open {
        svg_button("icons/collapse.svg", size, Some(style::collapse_svg))
        .on_press(Message::Pane(PaneMessage::StackCollapse(id, index)))
    } else {
        svg_button("icons/pane_terminal.svg", size, Some(style::collapse_svg_toggled))
        .on_press(Message::Pane(PaneMessage::StackExpand(id, index)))
    }.style(style::breakpoint)
}

fn pane_view_registers<'a>(pane: &'a PaneRegisters, id: pane_grid::Pane) -> Container<'a, Message> {
    fn flags(num: u64) -> String {
        let of = if num & (1 << 11) != 0 {"|OF"} else {""};
        let df = if num & (1 << 10) != 0 {"|DF"} else {""};
        let sf = if num & (1 << 7)  != 0 {"|SF"} else {""};
        let zf = if num & (1 << 6)  != 0 {"|ZF"} else {""};
        let af = if num & (1 << 4)  != 0 {"|AF"} else {""};
        let pf = if num & (1 << 2)  != 0 {"|PF"} else {""};
        let cf = if num & (1)       != 0 {"|CF"} else {""};
        let mut display = format!("{}{}{}{}{}{}{}", of, df, sf, zf, af, pf, cf);
        if display.len() > 0 {
            display.push('|');
        };
        display
    }


    let size = 30;

    let button_hex: button::Button<'_, Message> = button(
        text("0x").center().font(EXTRABOLD).size(size - 12)
        .style(if pane.format == Base::Hex {style::widget_text_toggled} else {style::widget_text})
    ).padding(0)
    .height(size)
    .width(size)
    .style(if pane.format == Base::Hex {style::widget_button_toggled} else {style::widget_button})
    .on_press(Message::Pane(PaneMessage::RegistersChangeFormat(id, Base::Hex)));

    let button_dec: button::Button<'_, Message> = button(
        text("10").center().font(EXTRABOLD).size(size - 12)
        .style(if pane.format == Base::Dec {style::widget_text_toggled} else {style::widget_text})
    ).padding(4)
    .height(size)
    .width(size)
    .style(if pane.format == Base::Dec {style::widget_button_toggled} else {style::widget_button})
    .on_press(Message::Pane(PaneMessage::RegistersChangeFormat(id, Base::Dec)));

    let button_oct: button::Button<'_, Message> = button(
        text("0o").center().font(EXTRABOLD).size(size - 12)
        .style(if pane.format == Base::Oct {style::widget_text_toggled} else {style::widget_text})
    ).padding(4)
    .height(size)
    .width(size)
    .style(if pane.format == Base::Oct {style::widget_button_toggled} else {style::widget_button})
    .on_press(Message::Pane(PaneMessage::RegistersChangeFormat(id, Base::Oct)));

    let button_bin: button::Button<'_, Message> = button(
        text("0b").center().font(EXTRABOLD).size(size - 12)
        .style(if pane.format == Base::Bin {style::widget_text_toggled} else {style::widget_text})
    ).padding(4)
    .height(size)
    .width(size)
    .style(if pane.format == Base::Bin {style::widget_button_toggled} else {style::widget_button})
    .on_press(Message::Pane(PaneMessage::RegistersChangeFormat(id, Base::Bin)));

    let regs = REGISTERS.access().clone();

    let (reg, value) = match regs {
        Some(regs) => ([
            "RIP:",
            "RAX:",
            "RBX:",
            "RCX:",
            "RDX:",
            "RSI:",
            "RDI:",
            "RBP:",
            "RSP:",
            "R8:",
            "R9:",
            "R10:",
            "R11:",
            "R12:",
            "R13:",
            "R14:",
            "R15:",
            "RFS:",
            "CS:",
            "SS:",
            "DS:",
            "ES:",
            "FS:",
            "GS:",
            "FSB:",
            "GSB:"
        ], [
        regs.rip,
        regs.rax,
        regs.rbx,
        regs.rcx,
        regs.rdx,
        regs.rsi,
        regs.rdi,
        regs.rbp,
        regs.rsp,
        regs.r8,
        regs.r9,
        regs.r10,
        regs.r11,
        regs.r12,
        regs.r13,
        regs.r14,
        regs.r15,
        regs.eflags,
        regs.cs,
        regs.ss,
        regs.ds,
        regs.es,
        regs.fs,
        regs.gs,
        regs.fs_base,
        regs.gs_base
        ]),
        None => return program_message("Start the program to display registers.")
    };

    let mut counter = 0;

    let reg_lines = column(reg.map(|name| text(name).center().size(18).wrapping(text::Wrapping::None).into()));
    let value_lines = column(value.map(|num|
        if counter == 17 {
            counter += 1;
            text(format!("{}   {}", pane.format.form(num), flags(num))).center().size(18).style(style::widget_text).wrapping(text::Wrapping::None).into()
        } else {
            counter += 1;
            text(pane.format.form(num)).center().size(18).style(style::widget_text).wrapping(text::Wrapping::None).into()
        }
    )).clip(true);


    let content = container(column![
        row![button_hex, button_dec, button_oct, button_bin].padding(3).spacing(3),
        scrollable(row![reg_lines, value_lines].padding(5).spacing(10)).width(Length::Fill).direction(scrollable::Direction::Both { vertical: scrollable::Scrollbar::new(), horizontal: scrollable::Scrollbar::new().scroller_width(0).width(0) })
    ]).style(style::back);
    content
}


fn pane_view_assembly<'a>(pane: &'a PaneAssembly, state: &'a State, id: pane_grid::Pane) -> Container<'a, Message> {
    if PID.access().is_none() {
        return program_message("Start the program to display assembly instructions.");
    }

    let size = 30;

    let rip = REGISTERS.access().unwrap().rip;

    let assembly = if let Some(assembly) = &state.internal.pane.assembly {
        let breakpoints = column(
            assembly.addresses.iter().map(|address| 
                breakpoint_button(Some(normal(*address)), size-5).into()
            )
        );

        let addresses = column(
            assembly.addresses.iter().map(|address|
                if address == &rip {
                    text(format!("0x{:06x}", address)).style(style::line).font(BOLD)
                } else {
                    text(format!("0x{:06x}", address)).style(style::weak)
                }.size(size-12).center().height(size-5).into()
            )
        );

        let bytes = text(&assembly.bytes)
        .size(size-12).line_height(iced::Pixels((size-5) as f32));

        let instructions = text(&assembly.text)
        .size(size-12).line_height(iced::Pixels((size-5) as f32));

        scrollable(
            row![
                breakpoints,
                addresses,
                container("").width(10),
                bytes,
                container("").width(10),
                instructions
            ]
        ).direction(scrollable::Direction::Both { vertical: scrollable::Scrollbar::new(), horizontal: scrollable::Scrollbar::new() })
        .id(pane.scrollable.clone())
        .height(Length::Fill)
        .width(Length::Fill)
    } else {
        return program_message("Assembly not loaded.")
    };
    container(
        assembly
    ).style(style::back)
}

pub fn assembly_scroll(state: &mut State, line: usize, task: &mut Option<Task<Message>>) {
    let panes = &state.layout.panes;
    let offset = scrollable::AbsoluteOffset { x: 0., y: (25.*(line as f32 - 3.)).max(0.)};

    for (_, pane) in panes.iter() {
        match pane {
            Pane::Assembly(data) => {
                *task = Some(scrollable::scroll_to(data.scrollable.clone(), offset))
            }
            _ => ()
        }
    };
}

pub fn check_for_assembly(state: &mut State) -> bool {
    for (_, pane) in state.layout.panes.iter() {
        match pane {
            Pane::Assembly(_) => return true,
            _ => ()
        }
    }
    false
}


fn pane_view_terminal<'a>(pane: &'a PaneTerminal, state: &'a State, id: pane_grid::Pane) -> Container<'a, Message> {
    let size = 20;

    if PID.access().is_none() {
        return program_message("Start the program to display the terminal.");
    }

    if STDIO.access().is_none() {
        return program_message("Terminal is set to external.");
    }

    let input = text_input("Input...", &pane.input)
    .size(size-5).line_height(iced::Pixels(size as f32))
    .on_input(move |text| Message::Pane(PaneMessage::TerminalType(id, text)))
    .on_paste(move |text| Message::Pane(PaneMessage::TerminalPaste(id, text)))
    .on_submit(Message::Pane(PaneMessage::TerminalSend(id)));

    let output = container(
        scrollable(
            text(format!("{}_", state.internal.pane.output)).size(size-5).line_height(0.95)
        ).direction(scrollable::Direction::Both { vertical: scrollable::Scrollbar::new(), horizontal: scrollable::Scrollbar::new() })
        .anchor_bottom()
        .anchor_left()
        .height(Length::Fill)
        .width(Length::Fill)
    ).padding(5)
    .style(style::terminal);


    container(column![
        output,
        input
    ].spacing(5)).style(style::back).padding(2)

}

fn pane_view_info<'a>() -> Container<'a, Message> {
    let bind = EHFRAME.access();
    if bind.is_none() {
        return program_message("Load the program to display ELF info.");
    };
    let file = match &bind.as_ref().unwrap().object {
        ::object::File::Elf64(elf) => elf,
        _ => panic!()
    };

    let elf_header = file.elf_header();
    let info = [
        ("Class:", InfoNamed::class(elf_header.e_ident.class)),
        ("Data:", InfoNamed::data(elf_header.e_ident.data)),
        ("Version:", &elf_header.e_ident.version.to_string()),
        ("OS/ABI", InfoNamed::os(elf_header.e_ident.os_abi)),
        ("ABI Version:", &elf_header.e_ident.abi_version.to_string()),
        ("Type:", InfoNamed::typ(elf_header.e_type.get(file.endian()))),
        ("Machine:", InfoNamed::machine(elf_header.e_machine.get(file.endian()))),
        ("Entry point address:", &format!("0x{:x}", elf_header.e_entry.get(file.endian()))),
        ("Program headers:", &format!("{} (offset into the file)", elf_header.e_phoff.get(file.endian()))),
        ("Section headers:", &format!("{} (offset into the file)", elf_header.e_shoff.get(file.endian()))),
    ];

    let size = 20;

    let field = column(info.iter().map(|(field, _)| {
        text(field.to_string()).size(size-5).center().height(size).wrapping(text::Wrapping::None).into()
    }));

    let value = column(info.iter().map(|(_, value)| {
        text(value.to_string()).size(size-5).center().height(size).style(style::widget_text).wrapping(text::Wrapping::None).into()
    }));

    let data = row![
        field, value
    ].padding(5).spacing(30);

    container(
        data
    ).width(Length::Fill).height(Length::Fill).style(style::back)

}

struct InfoNamed;

impl InfoNamed {
    fn class(x: u8) -> &'static str {
        match x {
            1 => "ELF32",
            2 => "ELF64",
            _ => "Invalid Class"
        }
    }

    fn data(x: u8) -> &'static str {
        match x {
            1 => "Little Endian",
            2 => "Big Endian",
            _ => "Unknown"
        }
    }

    fn os(x: u8) -> &'static str {
        match x {
            0 => "System V",
            1 => "HP-UX",
            2 => "NetBSD",
            3 => "Linux",
            4 => "GNU Hurd", // I did not miss 5, 5 is reserved :D
            6 => "Solaris",
            7 => "AIX",
            8 => "IRIX",
            9 => "FreeBSD",
            10 => "Tru64",
            11 => "Novell Modesto",
            12 => "OpenBSD",
            13 => "OpenVMS",
            14 => "NonStop Kernel",
            15 => "AROS",
            16 => "FenixOS",
            17 => "Nuxi CloudABI",
            18 => "Stratus Technologies OpenVOS",
            _ => "Unknow OS"
        }
    }

    fn typ(x: u16) -> &'static str {
        match x {
            1 => "REL (Relocatable file)",
            2 => "EXEC (Executable file)",
            3 => "DYN (Position-independent exec)",
            4 => "CORE (Core file.)",
            0xFE00|
            0xFEFF|
            0xFF00|
            0xFFFF => "Reserved.",
            _ => "Unknown Type"
        }
    }

    fn machine(x: u16) -> &'static str { // im using abbreviated names for simplicity
        match x {
            0 => "Unspecified",
            1 => "AT&T",
            0x02 => "SPARC",
            0x03 => "x86",
            0x06 => "Intel",
            0x3e => "AMD x86-64",
            _ => "Unknown or Unsupported Machine" // My program really supports only x86-64 so I dont have to write them out now
        }
    }
}



//PaneMessage Handle (Mainframe Operations)

fn get_pane<'a>(panes: &'a mut pane_grid::State<Pane>, pane: pane_grid::Pane) -> &'a mut Pane {
    panes.get_mut(pane).unwrap()
}

pub fn pane_message<'a>(state: &'a mut State, message: PaneMessage, task: &mut Option<Task<Message>>) {
    let panes = &mut state.layout.panes;

    match message {
        // Control
        PaneMessage::ControlSelectSignal(pane, signal) => get_pane(panes, pane).control().selected_signal = Some(signal),
        // Registers
        PaneMessage::RegistersChangeFormat(pane, base) => get_pane(panes, pane).registers().format = base,
        // Code
        PaneMessage::CodeSelectDir(pane, dir) => {
            let data = get_pane(panes, pane).code();
            data.viewport = None;
            if data.dir == Some(dir.clone()) {
                return;
            }
            data.dir = Some(dir);
            data.file = None;
        },
        PaneMessage::CodeSelectFile(pane, file) => {
            let data = get_pane(panes, pane).code();
            data.file = Some(file.clone()); //file select
            data.viewport = None;

            let comp_dir = PathBuf::from(match data.dir.clone() {
                Some(dir) => dir,
                None => return
            });
            let file_path = PathBuf::from(file.clone());
            let bind = SOURCE.access();
            let source = bind.as_ref().unwrap();
            let code = match source.get_file(comp_dir.clone(), file_path.clone()) {
                Some(file) => file,
                None => {
                    return;
                }
            };

            let index = code.1;
            if code.0.content.is_some() { //Conditional file load
                *task = Some(crate::trace::task_breapoints(
                    comp_dir,
                    index,
                    code.0.content.as_ref().unwrap().lines().count(),
                    pane
                ));
                return;
            };
            let mut path = comp_dir.clone();

            path.push(code.0.path.clone());

            let load = crate::trace::task_content(
                path,
                SourceIndex { line: 0, hash_path: comp_dir, index },
                Some(pane)
            );
            *task = Some(load)
        },
        PaneMessage::CodeLoad(pane, index, text) => {
            let count = text.lines().count();
            SOURCE.access().as_mut().unwrap()
            .index_mut(&index).content = Some(text);
            if let Some(pane) = pane {
                let load = crate::trace::task_breapoints(
                    index.hash_path,
                    index.index,
                    count,
                    pane
                );
                *task = Some(load)
            }
        }
        PaneMessage::CodeBreakpoints(pane, breakpoints) => {
            get_pane(panes, pane).code().breakpoints = breakpoints;
        },
        PaneMessage::CodeToggleUpdate(pane) => {
            let data = get_pane(panes, pane).code();
            data.update ^= true;
            if data.update && state.internal.pane.file.is_some() {
                let (scroll, load) = code_update(pane, state.internal.pane.file.as_ref().unwrap(), data);
                *task = Some(load.chain(scroll));
            }
        },
        PaneMessage::CodeScroll(pane, view) => get_pane(panes, pane).code().viewport = Some(view),
        // Memory
        PaneMessage::MemoryChangeFormat(pane, base) => get_pane(panes, pane).memory().format = base,
        PaneMessage::MemoryToggleSize(pane) => get_pane(panes, pane).memory().more_bytes ^= true,
        PaneMessage::MemoryInput(pane, data) => get_pane(panes, pane).memory().field = data,
        PaneMessage::MemoryPaste(pane, data) => get_pane(panes, pane).memory().field = data,
        PaneMessage::MemorySubmit(pane) => {
            let data = get_pane(panes, pane).memory();
            let field = &data.field;
            let hex = u64::from_str_radix(field.get(2..).unwrap_or("g"), 16);
            let dec = u64::from_str_radix(field, 10);
            if hex.is_err() && dec.is_err() {
                data.incorrect = true;
                return;
            };
            let num = match hex {
                Ok(num) => num,
                Err(_) => dec.unwrap()
            };
            data.address = num;
            data.incorrect = false;
            update_memory(data);
        },
        PaneMessage::MemoryAddress(pane, delta, mult) => {
            let data = get_pane(panes, pane).memory();
            let y = match delta {
                iced::mouse::ScrollDelta::Lines { x, y } => y*mult as f32,
                iced::mouse::ScrollDelta::Pixels { x, y } => y*mult as f32,
            };
            if data.more_bytes {
                let round = data.address % (8*mult.abs() as u64);
                if y > 0. {
                    data.address = data.address + 8*mult.abs() as u64 - (data.address % (8*mult.abs() as u64)) ;
                };
                if y < 0. {
                    if data.address < 8*mult.abs() as u64 {
                        data.address = 0;
                    } else {
                        if round != 0 {
                            data.address -= round;
                        } else {
                            data.address = data.address - 8*mult.abs() as u64;
                        }
                    }
                }
            } else {
                let round = data.address % (4*mult.abs() as u64);
                if y > 0. {
                    data.address = data.address + 4*mult.abs() as u64 - data.address % (4*mult.abs() as u64);
                };
                if y < 0. {
                    if data.address < 4*mult.abs() as u64 {
                        data.address = 0
                    } else {
                        if round != 0 {
                            data.address -= round;
                        } else {
                            data.address = data.address - (data.address %(4*mult.abs() as u64)) - 4*mult.abs() as u64;
                        }
                    }
                }
            };
            let hex_check = data.field.get(0..2);
            if hex_check.is_none() || hex_check.unwrap() == "0x" {
                data.field = format!("0x{:x}",data.address);
            } else {
                data.field = format!("{}",data.address);
            }
            data.incorrect = false;
            update_memory(data);
        },
        PaneMessage::MemoryReset(pane) => {
            let data = get_pane(panes, pane).memory();
            let mut beginning = 0;
            if state.internal.static_exec {
                for map in MAPS.access().as_ref().unwrap() {
                    if map.name != FILE.access().as_ref().unwrap().to_str().unwrap() {continue;}
                    if map.offset == 0 {
                        beginning = map.range.start
                    }
                };
            } else {
                beginning = anti_normal(0)
            };
            data.address = beginning;
            let hex_check = data.field.get(0..2);
            if hex_check.is_none() || hex_check.unwrap() == "0x" {
                data.field = format!("0x{:x}",data.address);
            } else {
                data.field = format!("{}",data.address);
            }
            data.incorrect = false;
            update_memory(data);
        },
        // Terminal
        PaneMessage::TerminalType(pane, data) => get_pane(panes, pane).terminal().input = data,
        PaneMessage::TerminalPaste(pane, data) => get_pane(panes, pane).terminal().input = data,
        PaneMessage::TerminalSend(pane) => {
            let data = get_pane(panes, pane).terminal();
            data.input.push('\n');

            if object::stdio().unwrap().write(data.input.as_bytes()).is_err() {
                return;
            };
            data.input.clear();
        },
        // Assembly
        PaneMessage::AssemblyUpdate(result) => {
            match result {
                Ok((assembly, line)) => {
                    state.internal.pane.assembly = Some(assembly);
                    assembly_scroll(state, line, task);
                }
                Err(()) => state.internal.pane.assembly = None
            }
        }
        // Stack
        PaneMessage::StackUpdate(pane) => {
            let data = get_pane(panes, pane).stack();
            data.unique = state.internal.pane.unique_stack;
            if state.internal.pane.stack.is_none() {return;}

            let stack = state.internal.pane.stack.as_ref().unwrap();
            let mut first = true;
            let mut open_new: Vec<bool> = stack.iter().rev().map(|(depth, _)| { // this maps all of the function lines to be shown, and the first function to be expanded
                if first {
                    if *depth == 0 {
                        first = false;
                    }
                    true
                } else {
                    *depth == 0
                }
            }).collect();

            open_new.reverse(); // because we iterated in reverse
            data.open = open_new;
        }
        PaneMessage::StackCollapse(pane, line) => {
            let data = get_pane(panes, pane).stack();
            let stack = state.internal.pane.stack.as_ref().unwrap();
            stack_open(stack, data, line, false);
        }
        PaneMessage::StackExpand(pane, line) => {
            let data = get_pane(panes, pane).stack();
            let stack = state.internal.pane.stack.as_ref().unwrap();
            stack_open(stack, data, line, true);
        }
        _ => ()
    };
}

pub fn source_content(file: PathBuf) -> Option<String> {
    match object::read_source(&file) {
        Ok(mut data) => Some({
            process_string(&mut data);
            data
        }),
        Err(()) => None
    }
}

pub fn create_breakpoints(comp_path: PathBuf, index: usize, len: usize) -> Vec<Option<u64>> {
    let bind = LINES.access();
    let lines = bind.as_ref().unwrap();

    let mut buf: Vec<Option<u64>> = Vec::new();

    let mut line = SourceIndex {
        line: 0,
        hash_path: comp_path,
        index
    };

    for i in 0..len {
        line.line = i as u64 + 1;
        buf.push(lines.get_address(&line));
    }

    buf
}

pub fn update_memory(pane: &mut PaneMemory) { // Works, Tested FR THIS TIME
    let current = pane.address;
    let limit = pane.read_address;

    match test_memory(current) {
        Err(true) => {
            pane.read_error = true;
            return;
        },
        _ => ()
    };

    let mut new = if (current < limit + 4096 / 8) || (current > limit + 7*(4096 / 8)) {
        (current - current % 8) - 2048 // radius
    } else {
        return; // we are still well within the bounds so no need to reload the data
    };

    match test_memory(new) {
        Err(true) => new = get_map_range(current).unwrap().start,
        _ => ()
    };

    match test_memory(new + 2048) {
        Err(false) => new = get_map_range(current).unwrap().end - 4096,
        _ => ()
    };

    if new == limit {
        return; // we have already done this
    };

    pane.read_address = new;

    let data = read_memory(new, 4096);
    if data.is_err() { // some error idk
        pane.read_error = true;
        return;
    }

    pane.read_error = false;
    pane.data = data.unwrap();
}

fn stack_open(stack: &Vec<(usize, String)>, pane: &mut PaneStack, line: usize, open: bool) {
    let upper = stack[line].0;
    let open_vec = &mut pane.open;
    for (i, (depth, _)) in stack.iter().skip(line+1).enumerate() {
        if *depth == upper {break;}
        open_vec[i+line+1] = open;
    };
}

// Layout Logic

pub fn layout_message(state: &mut State, message: LayoutMessage) {
    match message {
        LayoutMessage::SidebarLeftToggle => layout(&mut state.layout, message),
        LayoutMessage::SidebarRightToggle => layout(&mut state.layout, message),
        LayoutMessage::PanelToggle => layout(&mut state.layout, message),

        LayoutMessage::_Focus(pane) =>   state.layout._focus = Some(pane),
        LayoutMessage::Drag(pane_grid::DragEvent::Dropped {pane, target}) => {
            match target {
                pane_grid::Target::Pane(target_pane, _) => state.layout.panes.swap(pane, target_pane),
                //pane_grid::Target::Edge(edge) => state.layout.panes.drop(pane, pane_grid::Target::Edge(edge))
                _ => ()
            };
        }
        LayoutMessage::Resize(pane_grid::ResizeEvent {split, ratio}) => {
            //state.layout.panes.resize(split, ratio)
            resize(&mut state.layout, split, ratio);
        }
        //fill in later
        _ => ()
    };
}

fn layout(layout: &mut Layout, pane: LayoutMessage) {
    let (main, left, right, panel) = layout.get_nodes();
    let mut saved_state = SAVED_STATE.access().clone().unwrap();

    saved_state.main = Some(layout.node_to_configuration(&main));
    match left {
        Some(node) => saved_state.left_sidebar = (layout.node_to_configuration(&node.0), node.1),
        _ => ()
    };
    match right {
        Some(node) => saved_state.right_sidebar = (layout.node_to_configuration(&node.0), node.1),
        _ => ()
    };
    match panel {
        Some(node) => saved_state.panel = (layout.node_to_configuration(&node.0), node.1),
        _ => ()
    };

    match pane {
        LayoutMessage::SidebarLeftToggle => layout.sidebar_left ^= true,
        LayoutMessage::SidebarRightToggle => layout.sidebar_right ^= true,
        LayoutMessage::PanelToggle => {
            if layout.panel_mode == config::PanelMode::left {
                if layout.panel {
                    let new_left_ratio = (saved_state.left_sidebar.1 as f64)*(saved_state.right_sidebar.1 as f64);
                    let new_right_ratio = (1.0 - saved_state.right_sidebar.1 as f64)/(1.0 - new_left_ratio);
                    saved_state.left_sidebar.1 = new_left_ratio as f32;
                    saved_state.right_sidebar.1 = 1.0 - new_right_ratio as f32;
                } else {
                    let new_right_ratio = (1.0 - saved_state.right_sidebar.1 as f64)*(1.0 - saved_state.left_sidebar.1 as f64);
                    let new_left_ratio = (saved_state.left_sidebar.1 as f64)/(1.0 - new_right_ratio);
                    saved_state.left_sidebar.1 = new_left_ratio as f32;
                    saved_state.right_sidebar.1 = 1.0 - new_right_ratio as f32;
                }
            };
            layout.panel ^= true
        },
        _ => ()
    };

    SAVED_STATE.sets(saved_state.clone());

    let base = Layout::base(layout.sidebar_left, layout.sidebar_right, layout.panel, &layout.panel_mode, saved_state);

    layout.panes = pane_grid::State::with_configuration(base);
}

fn resize(layout: &mut Layout, split: pane_grid::Split, ratio: f32) { // big resize logic function (mainly because of sidebars)
    if layout.panel_mode == config::PanelMode::left && layout.panel {
        if !layout.sidebar_right {
            layout.panes.resize(split, ratio);
            return;
        }

        let (right, right_old_ratio) = layout.get_right_split();
        if split != right { // if we arent affecting the left split, then again normal configuration
            layout.panes.resize(split, ratio);
            return;
        }

        if layout.sidebar_right {
            let (left, left_old_ratio) = layout.get_left_split();
            let new_ratio = (left_old_ratio * right_old_ratio)/(ratio as f64);
            layout.panes.resize(right, limit(ratio));
            layout.panes.resize(left, limit(new_ratio as f32));
        } else {
            let left_old_ratio = SAVED_STATE.access().as_ref().unwrap().right_sidebar.1 as f64;
            let new_ratio = (left_old_ratio * right_old_ratio)/(ratio as f64);
            layout.panes.resize(right, limit(ratio));
            SAVED_STATE.access().as_mut().unwrap().left_sidebar.1 = limit(new_ratio as f32);
        };
        return;
    }

    if !layout.sidebar_left { //normal configuration if there inst left sidebar // if there is, then either update the right if it is there, OR update the global value
        layout.panes.resize(split, ratio);
        return;
    }

    let (left, left_old_ratio) = layout.get_left_split();
    if split != left { // if we arent affecting the left split, then again normal configuration
        layout.panes.resize(split, ratio);
        return;
    }

    if layout.sidebar_right {
        let (right, right_old_ratio) = layout.get_right_split();
        let new_ratio = (1.0 - (1.0 - (right_old_ratio * (1.0 - left_old_ratio)) - left_old_ratio) - ratio as f64)/(1.0 - ratio as f64);
        layout.panes.resize(left, limit(ratio));
        layout.panes.resize(right, limit(new_ratio as f32));
    } else {
        let right_old_ratio = SAVED_STATE.access().as_ref().unwrap().right_sidebar.1 as f64;
        let new_ratio = (1.0 - (1.0 - (right_old_ratio * (1.0 - left_old_ratio)) - left_old_ratio) - ratio as f64)/(1.0 - ratio as f64);
        layout.panes.resize(left, limit(ratio));
        SAVED_STATE.access().as_mut().unwrap().right_sidebar.1 = limit(new_ratio as f32);
    };
}
// NOTE: this is ofc only done for the sidebars, actually the correct way to do this for any pane is to find the next inner split (by recursing throught b) to find the first split that uses the SAME axis, then apply this logic for it. i might do that at some point

fn limit(ratio: f32) -> f32 { // this is not a user function, but to prevent some WEIRD graphics from appearing
    if ratio > 0.9 {
        0.9
    } else if  ratio < 0.1 {
        0.1
    } else {
        ratio
    }
}



// Widgets helpers

fn svg_button<'a>(icon: &str, size: u16, svg_style: Option<fn(&Theme, svg::Status) -> svg::Style>) -> button::Button<'a, Message> {
    button(
        svg(Handle::from_memory(Asset::get(icon).unwrap().data))
        .height(Length::Fill)
        .style(svg_style.unwrap_or(style::bar_svg))
    ).padding(4)
    .height(size)
    .width(size)
}

fn widget_fill<'a>() -> Container<'a, Message> {
    container("").width(Length::Fill).height(Length::Fill)
}

fn delimiter<'a>(width: usize) -> Container<'a, Message> {
    container("").width(Length::Fixed(width as f32)).height(Length::Fill)
}

fn program_message<'a>(msg: &'a str) -> Container<'a, Message> { // When the ui isnt active, use this container instead
    container(text(msg).center().height(Length::Fill).width(Length::Fill))
    .height(Length::Fill)
    .width(Length::Fill)
    .style(style::back)
}

pub fn process_string(file: &mut String) { // iced default font cannot display every character (even ones like tabs and such), therefore we replace them
    *file = file.chars().map(|char| match char {
        '\n' => "\n".to_string(),
        '\t' => "    ".to_string(),
        _ => if char.is_ascii_control() {"".to_string()} else {char.to_string()}
    }).collect()
}