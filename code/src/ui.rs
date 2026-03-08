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

use nix::sys::signal::Signal;

use crate::{
    window::*, data::*, trace::*, style, config, dwarf::*, object
};

const BOLD: font::Font = font::Font {weight: font::Weight::ExtraBold, ..font::Font::DEFAULT};

pub struct Layout {
    status_bar: bool,
    sidebar_left: bool,
    sidebar_right: bool,
    panel: bool,
    panel_mode: config::PanelMode,
    panes: pane_grid::State<Pane>,
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

const SIDERATIO: f32 = 0.25; // (0.1; 0.5)

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
                config::Pane::info => Pane::Info(PaneInfo::default()),
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

//    fn empty() -> Box<pane_grid::Configuration<Pane>> {
//        Box::new(pane_grid::Configuration::Pane(Pane::Empty))
//    }

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
enum Base {
    #[default]
    Hex,
    Dec,
    Oct,
    Bin,
}

impl Base {
    fn form(&self, num: u64) -> String {
        match self {
            Self::Hex => format!("0x{:x}", num),
            Self::Dec => format!("{}", num),
            Self::Oct => format!("0o{:o}", num),
            Self::Bin => format!("0b{:b}", num),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
enum ByteBase {
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
            Self::Chr => format!("'{:}'", if num.is_ascii_graphic() {num as char} else {' '}),
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
    Info(PaneInfo), // ELF dump
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
    fn info(&mut self) -> &mut PaneInfo {
        match self {
            Pane::Info(inner) => inner,
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
struct PaneMemory {
    field: String,
    incorrect: bool,
    address: u64, // where are we in memory, we read extra 2KB around this area and store to global data, and update only when we get outside of this region, for read effectivity
    read_address: u64, // where are we in memory, we read extra 2KB around this area and store to global data, and update only when we get outside of this region, for read effectivity
    data: Vec<u8>,
    more_bytes: bool, // 4 or 8
    format: ByteBase,
    read_error: bool // if read error occurs, show a button to take the user back (resets the address to a correct map)
}

#[derive(Debug, Clone, Default)]
struct PaneStack {} // TODO

#[derive(Debug, Clone, Default)]
struct PaneCode {
    update: bool,
    dir: Option<String>,
    file: Option<String>
}

#[derive(Debug, Clone, Default)]
struct PaneAssembly {} // TODO

#[derive(Debug, Clone, Default)]
struct PaneRegisters {
    pub format: Base
}

#[derive(Debug, Clone, Default)]
struct PaneInfo {} // TODO

#[derive(Debug, Clone, Default)]
struct PaneControl {} // TODO

#[derive(Debug, Clone, Default)]
struct PaneTerminal {
    text: String
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
    RegistersChangeFormat(pane_grid::Pane, Base),
    CodeSelectDir(pane_grid::Pane, String),
    CodeSelectFile(pane_grid::Pane, String),
    MemoryChangeFormat(pane_grid::Pane, ByteBase),
    MemoryToggleSize(pane_grid::Pane),
    MemoryInput(pane_grid::Pane, String),
    MemorySubmit(pane_grid::Pane),
    MemoryUpdate(pane_grid::Pane),
    MemoryPaste(pane_grid::Pane, String),
    MemoryAddress(pane_grid::Pane, iced::mouse::ScrollDelta, i8), //the i8 is as a signed multiplier, mirroring the axis
    MemoryReset(pane_grid::Pane)
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

fn statusbar<'a>(state: &State, height: usize) -> Container<'a, Message> {
    container(row![
        text("Program State | Program Position | Backtrace ...").size((height-7) as f32)
    ]).height(Length::Fixed(height as f32))
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
        Pane::Control(pane) => (pane_view_control(pane, state), pane_titlebar("Control", "icons/pane_control.svg")),
        Pane::Memory(pane) => (pane_view_memory(pane, id), pane_titlebar("Memory", "icons/pane_memory.svg")),
        Pane::Stack(pane) => (pane_view_stack(pane), pane_titlebar("CallStack", "icons/pane_stack.svg")),
        Pane::Registers(pane) => (pane_view_registers(pane, state, id), pane_titlebar("Registers", "icons/pane_registers.svg")),
        Pane::Assembly(pane) => (pane_view_assembly(pane), pane_titlebar("Assembly", "icons/pane_assembly.svg")),
        Pane::Terminal(pane) => (pane_view_terminal(pane), pane_titlebar("Terminal", "icons/pane_terminal.svg")),
        Pane::Info(pane) => (pane_view_info(pane), pane_titlebar("ELF Info", "icons/pane_info.svg")),

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
    fn breakpoint_button<'a>(address: Option<u64>) -> button::Button<'a, Message> {
        match address {
            Some(address) => {
                let present = BREAKPOINTS.access().as_ref().unwrap().contains_key(&address);
                button(
                svg(Handle::from_memory(Asset::get("icons/signal.svg").unwrap().data)).style(if present {style::breakpoint_svg_toggled} else {style::breakpoint_svg})
                ).style(style::breakpoint)
                .on_press(if present {Message::Operation(Operation::BreakpointRemove(address))} else {Message::Operation(Operation::BreakpointAdd(address))})
                .width(10)
                .height(10)
            },
            None => button("").style(style::breakpoint)
        }
        
    }


    let bind = SOURCE.access();
    if bind.is_none() {
        return program_message("Load the program to display source code.");
    };

    let source = bind.clone().unwrap();

    let mut dirs: Vec<String> = source.keys().map(|path| String::from(path.to_str().unwrap())).collect();

    dirs.sort();

    let hash_list: pick_list::PickList<'_, String, Vec<String>, String, Message> = pick_list(dirs, pane.dir.clone(), |path| Message::Pane(PaneMessage::CodeSelectDir(id, path))).placeholder("Dir...");

    let mut files: Vec<String> = source[&PathBuf::from(pane.dir.clone().unwrap())].iter().map(|file| String::from(file.path.to_str().unwrap())).collect();

    files.sort();

    let file_list: pick_list::PickList<'_, String, Vec<String>, String, Message> = pick_list(files, pane.file.clone(), |path| Message::Pane(PaneMessage::CodeSelectFile(id, path))).placeholder("File...");

    let comp_path = PathBuf::from(pane.dir.clone().unwrap());
    let file_path = PathBuf::from(pane.file.clone().unwrap());

    let (file, index) = source.get_file(comp_path.clone(), file_path).unwrap();
    let content = match &file.content {
        Some(text) => text,
        None => return program_message("File contents not loaded."),
    };

    let lines: Vec<&str> = content.lines().collect();
    let numbers: Vec<u64> = (0..lines.len()).map(|num| num as u64).collect();

    let addresses: Vec<Option<u64>> = numbers.iter().map(|num| {
        let source_index = SourceIndex {
            line: *num,
            hash_path: comp_path.clone(),
            index
        };
        let address = LINES.access().as_ref().unwrap().get_address(&source_index);
        address
    }).collect();

    let breakpoints: iced::widget::Column<'_, Message> = column(addresses.iter().map(|address|
        breakpoint_button(*address).into()
    ));

    let line_number: iced::widget::Column<'_, Message> = column(numbers.iter().map(|num| text(num).into()));
    let text: iced::widget::Column<'_, Message> = column(lines.iter().map(|line| text(*line).into()));
    container(
        column![]
    )
}

fn pane_view_control<'a>(pane: &PaneControl, state: &'a State) -> Container<'a, Message> {
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

    let step = svg_button("icons/step.svg", size, Some(if stopped {style::bar_svg} else {style::button_svg_disabled}))
    .on_press_maybe(if stopped {Some(Message::Operation(Operation::Step))} else {None})
    .style(style::widget_button);

    let source_step = svg_button("icons/source_step.svg", size, Some(if stopped {style::bar_svg} else {style::button_svg_disabled}))
    .on_press_maybe(if stopped {Some(Message::Operation(Operation::SourceStep))} else {None})
    .style(style::widget_button);

    let kill = svg_button("icons/signal_kill.svg", size, Some(if run {style::bar_svg} else {style::button_svg_disabled}))
    .on_press_maybe(if run {Some(Message::Operation(Operation::Kill))} else {None})
    .style(style::widget_button);

    let signal = svg_button("icons/signal.svg", size, Some(if run & state.internal.selected_signal.is_some() {style::bar_svg} else {style::button_svg_disabled}))
    .on_press_maybe(if run & state.internal.selected_signal.is_some() {Some(Message::Operation(Operation::Signal))} else {None})
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

    let select = pick_list(signals, state.internal.selected_signal, |signal| Message::Operation(Operation::SignalSelect(signal)))
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
        text("0x").center().font(BOLD).size(size - 12)
        .style(if pane.format == ByteBase::Hex {style::widget_text_toggled} else {style::widget_text})
    ).padding(0)
    .height(size)
    .width(size)
    .style(if pane.format == ByteBase::Hex {style::widget_button_toggled} else {style::widget_button})
    .on_press(Message::Pane(PaneMessage::MemoryChangeFormat(id, ByteBase::Hex)));

    let button_dec: button::Button<'_, Message> = button(
        text("10").center().font(BOLD).size(size - 12)
        .style(if pane.format == ByteBase::Dec {style::widget_text_toggled} else {style::widget_text})
    ).padding(0)
    .height(size)
    .width(size)
    .style(if pane.format == ByteBase::Dec {style::widget_button_toggled} else {style::widget_button})
    .on_press(Message::Pane(PaneMessage::MemoryChangeFormat(id, ByteBase::Dec)));

    let button_chr: button::Button<'_, Message> = button(
        text("A").center().font(BOLD).size(size - 12)
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

    let memory = if false {
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
                format!("0x...{:06x}", addr)
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
        ).into()))
        .spacing(match pane.format {
            ByteBase::Hex => 10,
            ByteBase::Chr => 15,
            ByteBase::Dec => 20
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

fn pane_view_stack<'a>(pane: &PaneStack) -> Container<'a, Message> {
    let content = container(text("TERMINAL"));
    content
}

fn pane_view_registers<'a>(pane: &PaneRegisters, state: &'a State, id: pane_grid::Pane) -> Container<'a, Message> {
    fn flags(num: u64) -> String {
        let mut buf = String::new();
        if num & (1 << 11) != 0 {
            buf.push_str("|OF");
        };
        if num & (1 << 10) != 0 {
            buf.push_str("|DF");
        };
        if num & (1 << 7) != 0 {
            buf.push_str("|SF");
        };
        if num & (1 << 6) != 0 {
            buf.push_str("|ZF");
        };
        if num & (1 << 4) != 0 {
            buf.push_str("|AF");
        };
        if num & (1 << 2) != 0 {
            buf.push_str("|PF");
        };
        if num & (1) != 0 {
            buf.push_str("|CF");
        };

        if buf.len() > 0 {
            buf.push('|');
        };
        buf
    }

    let size = 30;

    let button_hex: button::Button<'_, Message> = button(
        text("0x").center().font(BOLD).size(size - 12)
        .style(if pane.format == Base::Hex {style::widget_text_toggled} else {style::widget_text})
    ).padding(0)
    .height(size)
    .width(size)
    .style(if pane.format == Base::Hex {style::widget_button_toggled} else {style::widget_button})
    .on_press(Message::Pane(PaneMessage::RegistersChangeFormat(id, Base::Hex)));

    let button_dec: button::Button<'_, Message> = button(
        text("10").center().font(BOLD).size(size - 12)
        .style(if pane.format == Base::Dec {style::widget_text_toggled} else {style::widget_text})
    ).padding(4)
    .height(size)
    .width(size)
    .style(if pane.format == Base::Dec {style::widget_button_toggled} else {style::widget_button})
    .on_press(Message::Pane(PaneMessage::RegistersChangeFormat(id, Base::Dec)));

    let button_oct: button::Button<'_, Message> = button(
        text("0o").center().font(BOLD).size(size - 12)
        .style(if pane.format == Base::Oct {style::widget_text_toggled} else {style::widget_text})
    ).padding(4)
    .height(size)
    .width(size)
    .style(if pane.format == Base::Oct {style::widget_button_toggled} else {style::widget_button})
    .on_press(Message::Pane(PaneMessage::RegistersChangeFormat(id, Base::Oct)));

    let button_bin: button::Button<'_, Message> = button(
        text("0b").center().font(BOLD).size(size - 12)
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

fn pane_view_assembly<'a>(pane: &PaneAssembly) -> Container<'a, Message> {
    let content = container(text("TERMINAL"));
    content
}

fn pane_view_terminal<'a>(pane: &PaneTerminal) -> Container<'a, Message> {
    let content = container(text("TERMINAL"));
    content
}

fn pane_view_info<'a>(pane_grid: &PaneInfo) -> Container<'a, Message> {
    let content = container(text("INFO"));
    content
}




//message Handle

pub fn pane_message<'a>(state: &'a mut State, message: PaneMessage) {
    let panes = &mut state.layout.panes;
    let get_pane = |panes: &'a mut pane_grid::State<Pane>, pane: pane_grid::Pane| panes.get_mut(pane).unwrap();

    match message {
        PaneMessage::RegistersChangeFormat(pane, base) => get_pane(panes, pane).registers().format = base,
        PaneMessage::CodeSelectDir(pane, dir) => get_pane(panes, pane).code().dir = Some(dir),
        PaneMessage::CodeSelectFile(pane, file) => {
            let data = get_pane(panes, pane).code();
            data.file = Some(file.clone()); //file select

            let comp_dir = PathBuf::from(data.dir.clone().unwrap());
            let file_path = PathBuf::from(file);
            let bind = SOURCE.access();
            let source = bind.as_ref().unwrap();
            let code = source.get_file(comp_dir.clone(), file_path.clone()).unwrap();
            if code.0.content.is_some() { //Conditional file load
                return;
            };
            let index = code.1;
            drop(bind);
            let mut path = comp_dir.clone();
            path.push(file_path);
            if let Ok(file) = object::read_source(&path) {
                let mut bind = SOURCE.access();
                let source = bind.as_mut().unwrap();
                source.get_mut(&comp_dir).unwrap().get_mut(index).unwrap().content = Some(file);
            };
        },
        PaneMessage::MemoryChangeFormat(pane, base) => get_pane(panes, pane).memory().format = base,
        PaneMessage::MemoryToggleSize(pane) => get_pane(panes, pane).memory().more_bytes ^= true,
        PaneMessage::MemoryInput(pane, data) => get_pane(panes, pane).memory().field = data,
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
            data.address = anti_normal(0);
            let hex_check = data.field.get(0..2);
            if hex_check.is_none() || hex_check.unwrap() == "0x" {
                data.field = format!("0x{:x}",data.address);
            } else {
                data.field = format!("{}",data.address);
            }
            data.incorrect = false;
            update_memory(data);
        },
        _ => ()
    };
}

fn update_memory(pane: &mut PaneMemory) {
    let current = pane.address;
    let limit = pane.read_address;

    match test_memory(current) {
        Err(true) => {
            pane.data.clear();
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
        Err(false) => new = get_map_range(current).unwrap().end - 4096,
        Ok(()) => ()
    };

    if new == limit {
        return; // we have already done this
    };

    pane.read_address = new;

    let data = read_memory(new, 4096);
    if data.is_err() { // some error idk
        pane.data.clear();
        pane.read_error = true;
        return;
    }

    pane.read_error = false;
    pane.data = data.unwrap();
}


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
        //println!("{new_ratio}");
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
    container(text("|")).width(Length::Fixed(width as f32)).height(Length::Fill)
}

fn program_message<'a>(msg: &'a str) -> Container<'a, Message> {
    container(text(msg).center().height(Length::Fill).width(Length::Fill))
    .height(Length::Fill)
    .width(Length::Fill)
    .style(style::back)
}