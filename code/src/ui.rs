use iced::{
    Task,
    Length,
    widget::{
        Container,
        Row,
        Theme,
        svg::Handle,
        text,
        container,
        button,
        column,
        row,

        svg,
        pane_grid,
        pick_list,
        scrollable,
        text_input,
        mouse_area
    },
    padding,
    font
};

use std::{
    path::PathBuf,
    io::Write
};

use nix::sys::signal::Signal;

use ::object as object_foreign;

// internal import
use crate::{
    window::*,
    data::*,
    trace::*,
    dwarf::*,
    style,
    config,
    object
};


// FILE: ui.rs - Creating and Handling the user interface and graphics

// Fonts
const EXTRABOLD: font::Font = font::Font {weight: font::Weight::ExtraBold, ..font::Font::DEFAULT};
const BOLD: font::Font = font::Font {weight: font::Weight::Bold, ..font::Font::DEFAULT};

// PaneGrid Layout
const SIDERATIO: f32 = 0.25; // (0.1; 0.4)      // Default ratio of sidebars

pub struct Layout { // state of the Mainframe
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
        let layout = CONFIG.access().as_ref().unwrap().layout.clone().unwrap(); // load from config
        Layout {
            status_bar: *layout.status_bar.as_ref().unwrap(),
            sidebar_left: *layout.sidebar_left.as_ref().unwrap(),
            sidebar_right: *layout.sidebar_right.as_ref().unwrap(),
            panel: *layout.panel.as_ref().unwrap(),
            panel_mode: *layout.panel_mode.as_ref().unwrap(),
            panes: Self::panes_config(&layout),
            _focus: None
        }
    }
}

impl Layout {
    fn panes_config(layout: &config::Layout) -> pane_grid::State<Pane> { // creating panes from config
        let left = layout.sidebar_left.unwrap();
        let right = layout.sidebar_right.unwrap();
        let panel = layout.panel.unwrap();
        let panel_mode = layout.panel_mode.as_ref().unwrap();

        let (left_ratio, right_ratio) = match panel_mode {
            config::PanelMode::left => { // only panel left with panel is opposite calculation
                if panel {
                    ((SIDERATIO)/(1.0-SIDERATIO), (1.0 - SIDERATIO)) // converting percantages ...
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

        let panes = SavedState { // we create the pane configuration of each part
            left_sidebar: (Self::serialize(list.left, pane_grid::Axis::Horizontal), left_ratio),
            right_sidebar: (Self::serialize(list.right, pane_grid::Axis::Horizontal), right_ratio),
            panel: (Self::serialize(list.panel, pane_grid::Axis::Vertical), panel_ratio),
            main: Some(Self::serialize(list.main, pane_grid::Axis::Vertical))
        };

        SAVED_STATE.sets(panes.clone()); // we save this configuration

        let base = Self::base(left, right, panel, &panel_mode, panes); // we create the final layout from the parts

        pane_grid::State::with_configuration(base) // state from configuration
    }

    fn base(left: bool, right: bool, panel: bool, panel_mode: &config::PanelMode, panes: crate::data::SavedState) -> pane_grid::Configuration<Pane> {
        let left_ratio = panes.left_sidebar.1;
        let right_ratio = panes.right_sidebar.1;

        if panel { return match panel_mode { // if panel, then we match panel type and call respective function
            config::PanelMode::full => Self::panel_full(left, right, panes),
            config::PanelMode::middle => Self::panel_middle(left, right, panes),
            config::PanelMode::left => Self::panel_left(left, right, panes),
            config::PanelMode::right => Self::panel_right(left, right, panes)
        };} else { //without the panel
            if left & right { // if both, then first we split the LEFT and MAIN, and then we split RIGHT from MAIN
                return pane_grid::Configuration::Split{
                    axis: pane_grid::Axis::Vertical,
                    ratio: left_ratio,
                    a: Box::new(panes.left_sidebar.0),
                    b: Box::new(pane_grid::Configuration::Split {
                    axis: pane_grid::Axis::Vertical,
                    ratio: right_ratio,
                    a: Box::new(panes.main.unwrap()),
                    b: Box::new(panes.right_sidebar.0)
                })};
            };
            if left { // if LEFT, then just LEFT and MAIN
                return pane_grid::Configuration::Split {
                    axis: pane_grid::Axis::Vertical,
                    ratio: left_ratio,
                    a: Box::new(panes.left_sidebar.0),
                    b: Box::new(panes.main.unwrap())
                };
            };
            if right { // if RIGHT, then just MAIN and RIGHT
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
            return pane_grid::Configuration::Split { // Same logic as if without panel, but from panel_mode we determine the order
                axis: pane_grid::Axis::Horizontal,
                ratio: panel_ratio,
                b: Box::new(panes.panel.0),
                a: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: left_ratio,
                a: Box::new(panes.left_sidebar.0),
                b: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: right_ratio,
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
            return pane_grid::Configuration::Split{ // Same logic as if without panel, but from panel_mode we determine the order
                axis: pane_grid::Axis::Vertical,
                ratio: left_ratio,
                a: Box::new(panes.left_sidebar.0),
                b: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: right_ratio,
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
            return pane_grid::Configuration::Split { // Only here we first have to create the RIGHT, and then MAIN and PANEL, and then split LEFT from MAIN (thats why mirrored ratios)
                axis: pane_grid::Axis::Vertical,
                ratio: right_ratio,
                b: Box::new(panes.right_sidebar.0),
                a: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Horizontal,
                ratio: panel_ratio,
                b: Box::new(panes.panel.0),
                a: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: left_ratio,
                a: Box::new(panes.left_sidebar.0),
                b: Box::new(panes.main.unwrap()),
            })})};
        };
        if left { // Normal Logic
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
            return pane_grid::Configuration::Split { // Same logic as if without panel, but from panel_mode we determine the order
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


    fn serialize(mut list: Vec<config::Pane>, axis: pane_grid::Axis) -> pane_grid::Configuration<Pane> { // turning config array of panes, into Configuration
        fn magic(current: u8) -> f32 { // now when creating evenly distributed panes, we follow a rule of essentially just making 1/(n+1) series, where n is the amount still left
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
                ratio: magic(list.len() as u8),
                a: Box::new(pane_grid::Configuration::Pane(pane)),
                b: Box::new(Self::serialize(list, axis))
            }
        }
    }

    fn get_left_split(&self) -> (pane_grid::Split, f64) { // caller must know that the side bar IS ACTUALLY ACTIVE // function to find the LEFT sidebar based on layout state
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

    fn get_right_split(&self) -> (pane_grid::Split, f64) { // caller must know that BOTH sidebars ARE ACTUALLY ACTIVE // function to find the RIGHT sidebar based on layout state
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
        #[allow(unused_assignments)]
        let mut main = None;
        let mut left = None;
        let mut right = None;
        let mut panel = None;

        let layout = self.panes.layout().clone();

        let random: Option<pane_grid::Split> = match layout.splits().next() {
            Some(split) => Some(split.clone()),
            None => None
        };

        if self.panel { match self.panel_mode { // we reverse the same rules as when defining the base (A lot of indents because the nodes are a recursive Enum and we need to match it)
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

        (main.unwrap(), left, right, panel) // main is always there
    }

    fn node_to_configuration(&self, node: &pane_grid::Node) -> pane_grid::Configuration<Pane> { // a recursive function, as this is the easiest way
        match node {
            pane_grid::Node::Pane(pane) => pane_grid::Configuration::Pane(self.panes.get(*pane).unwrap().clone()), //retrive the state of the pane
            pane_grid::Node::Split { id: _, axis, ratio, a, b } => {
                pane_grid::Configuration::Split { // creating the configuration recursively
                    axis: *axis,
                    ratio: *ratio,
                    a: Box::new(self.node_to_configuration(a)), // recursive calls
                    b: Box::new(self.node_to_configuration(b)) // recursive calls
                }
            }
        }
    }
}

// Panes and view functions

#[derive(Debug, Clone)]
pub enum Pane { // Generic enum for all bars (completed widgets that can be moved around inside a window)
    Control(PaneControl),
    Registers(PaneRegisters),
    Memory(PaneMemory),
    Code(PaneCode),
    Info, // ELF dump
    Terminal(PaneTerminal),
    Stack(PaneStack),
    Assembly(PaneAssembly),
    _Empty
}

impl Pane { // to avoid matching when we know which pane we are working with
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
}

// Each pane has its own struct (its state), and has a view() function that retrieves the graphics of the pane
#[derive(Debug, Clone, Default)]
pub struct PaneControl {
    selected_signal: Option<Signal>,
}
impl PaneControl {
    fn view<'a>(&self, state: &'a State, id: pane_grid::Pane) -> Container<'a, Message> {
        let size = 30;

        // loading the states
        let file = FILE.access().is_some();
        let run = PID.access().is_some();
        let stopped = state.internal.stopped;

        // creating the buttons based on the state with different messages and SVGs
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
            if stopped & !state.internal.no_debug & state.last_signal.is_none() { // last signal with source step produces an error, so we force the user to first step to get rid of the signal
                Some(Message::Operation(Operation::SourceStep))
            } else {None}
        ).style(style::widget_button);

        // kill and signal buttons just set the last signal to the desired signal, delivered on continue or step
        let kill = svg_button("icons/signal_kill.svg", size, Some(if run {style::widget_svg} else {style::button_svg_disabled}))
        .on_press_maybe(if run {Some(Message::Operation(Operation::Kill))} else {None})
        .style(style::widget_button);

        let signal = svg_button("icons/signal.svg", size, Some(if self.selected_signal.is_some() {style::widget_svg} else {style::button_svg_disabled}))
        .on_press_maybe(if self.selected_signal.is_some() {Some(Message::Operation(Operation::Signal(self.selected_signal.unwrap())))} else {None})
        .style(style::widget_button);

        let signals = [ // Signals to select
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

        let select = pick_list(signals, self.selected_signal, move |signal| Message::Pane(PaneMessage::ControlSelectSignal(id, signal)))
        .placeholder("Signal...");

        let content = container(row![ // row of buttons
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
}

#[derive(Debug, Clone, Default)]
pub struct PaneRegisters {
    format: Base
}
impl PaneRegisters {
    fn view<'a>(&self, id: pane_grid::Pane) -> Container<'a, Message> {

        fn flags(num: u64) -> String { // creating visual flags from set bits in the RFLAGS
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

        // display format buttons
        let button_hex: button::Button<'_, Message> = button(
            text("0x").center().font(EXTRABOLD).size(size - 12)
            .style(if self.format == Base::Hex {style::widget_text_toggled} else {style::widget_text})
        ).padding(0)
        .height(size)
        .width(size)
        .style(if self.format == Base::Hex {style::widget_button_toggled} else {style::widget_button})
        .on_press(Message::Pane(PaneMessage::RegistersChangeFormat(id, Base::Hex)));

        let button_dec: button::Button<'_, Message> = button(
            text("10").center().font(EXTRABOLD).size(size - 12)
            .style(if self.format == Base::Dec {style::widget_text_toggled} else {style::widget_text})
        ).padding(4)
        .height(size)
        .width(size)
        .style(if self.format == Base::Dec {style::widget_button_toggled} else {style::widget_button})
        .on_press(Message::Pane(PaneMessage::RegistersChangeFormat(id, Base::Dec)));

        let button_oct: button::Button<'_, Message> = button(
            text("0o").center().font(EXTRABOLD).size(size - 12)
            .style(if self.format == Base::Oct {style::widget_text_toggled} else {style::widget_text})
        ).padding(4)
        .height(size)
        .width(size)
        .style(if self.format == Base::Oct {style::widget_button_toggled} else {style::widget_button})
        .on_press(Message::Pane(PaneMessage::RegistersChangeFormat(id, Base::Oct)));

        let button_bin: button::Button<'_, Message> = button(
            text("0b").center().font(EXTRABOLD).size(size - 12)
            .style(if self.format == Base::Bin {style::widget_text_toggled} else {style::widget_text})
        ).padding(4)
        .height(size)
        .width(size)
        .style(if self.format == Base::Bin {style::widget_button_toggled} else {style::widget_button})
        .on_press(Message::Pane(PaneMessage::RegistersChangeFormat(id, Base::Bin)));

        let regs = REGISTERS.access().clone();

        let (reg, value) = match regs { // two lists of reg_names and reg_values
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
            None => return program_message("Start the program to display registers.") // message if None
        };

        let mut counter = 0;

        let reg_lines = column(reg.map(|name| //names
            text(name).center().size(size - 12).wrapping(text::Wrapping::None).into()
        ));
        let value_lines = column(value.map(|num| // values
            if counter == 17 { // display flags next to the RFLAGS register
                counter += 1;
                text(format!("{}   {}", self.format.form(num), flags(num)))
                .center()
                .size(size - 12)
                .style(style::widget_text)
                .wrapping(text::Wrapping::None)
                .into()
            } else {
                counter += 1;
                text(self.format.form(num))
                .center()
                .size(size - 12)
                .style(style::widget_text)
                .wrapping(text::Wrapping::None)
                .into()
            }
        )).clip(true);


        let content = container(column![
            row![button_hex, button_dec, button_oct, button_bin].padding(3).spacing(3),
            scrollable(
                row![reg_lines, value_lines]
                .padding(5).spacing(10)
            ).width(Length::Fill)
            .direction(scrollable::Direction::Both { vertical: scrollbar(), horizontal: no_scrollbar() })
        ]).style(style::back);
        content
    }

}

#[derive(Debug, Clone, Default)]
pub struct PaneMemory {
    pub field: String,
    incorrect: bool,
    pub address: u64, // where are we in memory, we read extra 2KB around this area and store to global data, and update only when we get outside of this region, for read effectivity
    read_address: u64, // the actual address of the last read
    data: Vec<u8>, // the 4KB of data
    more_bytes: bool, // 4 or 8
    format: ByteBase,
    read_error: bool // if read error occurs, show a button to take the user back (resets the address to a correct map)
}
impl PaneMemory {
    fn view<'a>(&'a self, id: pane_grid::Pane) -> Container<'a, Message> {
        if MEMORY.access().is_none() {
            return program_message("Start the program to display memory.");
        };

        let size = 30;

        // display format buttons
        let button_hex: button::Button<'_, Message> = button(
            text("0x").center().font(EXTRABOLD).size(size - 12)
            .style(if self.format == ByteBase::Hex {style::widget_text_toggled} else {style::widget_text})
        ).padding(0)
        .height(size)
        .width(size)
        .style(if self.format == ByteBase::Hex {style::widget_button_toggled} else {style::widget_button})
        .on_press(Message::Pane(PaneMessage::MemoryChangeFormat(id, ByteBase::Hex)));

        let button_dec: button::Button<'_, Message> = button(
            text("10").center().font(EXTRABOLD).size(size - 12)
            .style(if self.format == ByteBase::Dec {style::widget_text_toggled} else {style::widget_text})
        ).padding(0)
        .height(size)
        .width(size)
        .style(if self.format == ByteBase::Dec {style::widget_button_toggled} else {style::widget_button})
        .on_press(Message::Pane(PaneMessage::MemoryChangeFormat(id, ByteBase::Dec)));

        let button_chr: button::Button<'_, Message> = button(
            text("A").center().font(EXTRABOLD).size(size - 12)
            .style(if self.format == ByteBase::Chr {style::widget_text_toggled} else {style::widget_text})
        ).padding(0)
        .height(size)
        .width(size)
        .style(if self.format == ByteBase::Chr {style::widget_button_toggled} else {style::widget_button})
        .on_press(Message::Pane(PaneMessage::MemoryChangeFormat(id, ByteBase::Chr)));

        let bytesize: button::Button<'_, Message> = button(
            text(if self.more_bytes {"8"} else {"4"}).center().size(size - 12).style(style::widget_text)
        )
        .padding(0)
        .height(size)
        .width(size)
        .style(style::widget_button)
        .on_press(Message::Pane(PaneMessage::MemoryToggleSize(id)));

        // address field
        let address: text_input::TextInput<'_, Message> = text_input("0x...", &self.field)
        .on_input(move |data| Message::Pane(PaneMessage::MemoryInput(id, data)))
        .on_submit(Message::Pane(PaneMessage::MemorySubmit(id)))
        .on_paste(move |data| Message::Pane(PaneMessage::MemoryPaste(id, data)))
        .size(size - 12)
        .line_height(iced::Pixels(size as f32 - 10.))
        .width(Length::Fill)
        .style(|theme, status| style::address(theme, status, self.incorrect));

        let field = row![
            text("Address:").size(size - 12).center().height(size),
            container(
                mouse_area(address) // mouse_area, so its interactive
                .on_scroll(move |delta| Message::Pane(PaneMessage::MemoryAddress(id, delta, 1)))).width(Length::FillPortion(4)
            ),
            widget_fill(),
            button_hex,
            button_dec,
            button_chr,
            bytesize
        ].spacing(2).padding(3).height(Length::Shrink);

        let test = match test_memory(self.address) {
            Err(test) => test,
            Ok(()) => false
        };

        let memory = if test { // out of bounds message, and button to reload
            container(column![
                text("Read out of memory map bounds.")
                .width(Length::Fill).center().style(style::error),
                container(
                    button(text("Reload to begining of program map"))
                    .on_press(Message::Pane(PaneMessage::MemoryReset(id)))
                ).width(Length::Fill).center_x(Length::Fill)
            ]).center(Length::Fill).width(Length::Fill).height(Length::Fill)
        } else {
            let data: &Vec<u8>= &self.data; // bytes

            let mut addresses: Vec<u64> = Vec::new();
            let mut bytes: Vec<Vec<u8>> = if self.more_bytes { // creating the bytes columns
                vec![Vec::new(); 8]
            } else {
                vec![Vec::new(); 4]
            };

            let len = bytes.len();
            let mut pointer = self.address - self.address % len as u64; // address
            let start = pointer - self.read_address; // start of the displayed bytes

            for (i, byte) in data[start as usize..].iter().enumerate() { // iterating through the bytes, pushing the address every line
                if i % len == 0 {
                    addresses.push(pointer);
                    pointer += len as u64;
                };

                bytes[i % len].push(*byte); // cycling the columns, incrementing the depth each lines
                if i == 40*len - 1 { // 40 lines (more cant fit on the screen)
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

            let byte_columns: iced::widget::Row<'_, Message> = row(bytes.iter().map(|col| column( // we create columns seperately for nicer alignment
                col.iter().map(|byte| text(
                    self.format.form(*byte) // formatting the bytes
                ).size(size - 5)
                .height(size)
                .center()
                .style(style::widget_text)
                .into())
            ).align_x(iced::Alignment::Center)
            .width(match self.format {
                ByteBase::Chr => Length::Fixed(20 as f32),
                _ => Length::Shrink
            })
            .into())
            ).spacing(match self.format { // custom spacing (each format takes up a different size)
                ByteBase::Hex => 10,
                ByteBase::Chr => 10,
                ByteBase::Dec => 15
            });

            container(mouse_area( // for scrolling the contents
                row![address_column, byte_columns]
                .height(Length::Fill)
                .width(Length::Fill)
                .padding(5)
                .spacing(20)
            ).on_scroll(move |delta| Message::Pane(PaneMessage::MemoryAddress(id, delta, -3)))) // same message, bigger increment and negatives
        };

        let content = container(column![
            field,
            memory
        ]).style(style::back);
        content
    }
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
impl PaneCode {
    fn view<'a>(&self, state: &'a State, id: pane_grid::Pane) -> Container<'a, Message> {
        if state.internal.no_debug {
            return program_message("No debugging informatio")
        }

        let size = 30;

        let update_button = svg_button( // to determine whether to display the file and position where we are stopped at
            "icons/view.svg",
            size,
            Some(if self.update {style::widget_svg_toggled} else {style::widget_svg}))
        .style(if self.update {style::widget_button_toggled} else {style::widget_button})
        .on_press(Message::Pane(PaneMessage::CodeToggleUpdate(id)));

        let bind = SOURCE.access();
        if bind.is_none() {
            if state.internal.no_debug {
                return program_message("No debugging information present in the file.");
            }
            return program_message("Load the program to display source code.");
        };

        let source = bind.clone().unwrap();

        let mut dirs: Vec<String> = source.keys().map(|path| String::from(path.to_str().unwrap())).collect(); // list of source dirs from the sourcemap
        dirs.sort();
        dirs.dedup(); // remove duplicates (because of different compilation units)

        let hash_list: pick_list::PickList<'_, String, Vec<String>, String, Message> = pick_list(dirs.clone(), self.dir.clone(), move |path| Message::Pane(PaneMessage::CodeSelectDir(id, path)));

        let mut files: Vec<String> = source[ //list of files from the source dir
            &PathBuf::from(self.dir.clone().unwrap_or(dirs[0].clone()))
        ].iter().map(|file| String::from(file.path.to_str().unwrap())).collect();
        files.sort();
        files.dedup(); // remove duplicates (because of different compilation units)

        let file_list: pick_list::PickList<'_, String, Vec<String>, String, Message> = pick_list(files, self.file.clone(), move |path| Message::Pane(PaneMessage::CodeSelectFile(id, path)));

        let code = if self.dir.is_some() {match &self.file {
            Some(file) => {
                let comp_path = PathBuf::from(self.dir.as_ref().unwrap());
                let file_path = PathBuf::from(file);
                let code = self.code_display(comp_path, file_path, source, &state.internal.pane.file); // err if not contents
                if code.is_ok() {
                    container(scrollable(
                    code.unwrap()
                    ).direction(scrollable::Direction::Both { vertical: scrollbar(), horizontal: scrollbar() })
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .id(self.scrollable.clone())
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
                row![
                    hash_list,
                    file_list,
                    widget_fill(),
                    update_button
                ].spacing(10).padding(3).height(size+6),
                code
            ]
        ).style(style::back)
    }

    fn code_display<'a>(&self, comp_path: PathBuf, file_path: PathBuf, source: SourceMap, line: &Option<SourceIndex>) -> Result<Row<'a, Message>, ()> {

        let (file, _index) = match source.get_file(comp_path.clone(), file_path.clone()) {
            Some(file) => file,
            None => return Err(())
        };
        if file.content.is_none() {
            return Err(())
        }

        let size = 25;

        let mut lines = Vec::new();
        let breakpoints = column(
            self.breakpoints.iter().enumerate().map(|(index, address)| {
                lines.push(index);
                breakpoint_button(*address, size).into()
            })
        );

        let highlight = match line { // getting the current line, if we are in the correct source file
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

struct PaneInfo;
impl PaneInfo {
    fn view<'a>() -> Container<'a, Message> {
    let bind = EHFRAME.access();
    if bind.is_none() {
        return program_message("Load the program to display ELF info.");
    };
    let file = match &bind.as_ref().unwrap().object { // we create the object file from the bytes, to get the formating data of the ELF header
        object_foreign::File::Elf64(elf) => elf,
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
        text(field.to_string())
        .size(size-5)
        .center()
        .height(size)
        .wrapping(text::Wrapping::None).into()
    }));

    let value = column(info.iter().map(|(_, value)| {
        text(value.to_string())
        .size(size-5)
        .center()
        .height(size)
        .style(style::widget_text)
        .wrapping(text::Wrapping::None).into()
    }));

    let data = row![
        field, value
    ].padding(5).spacing(30);

    container(
        data
    ).width(Length::Fill).height(Length::Fill).style(style::back)
}
}
struct InfoNamed; // for displaying the ELF header
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

#[derive(Debug, Clone, Default)]
pub struct PaneTerminal {
    input: String
}
impl PaneTerminal {
    fn view<'a>(&self, state: &'a State, id: pane_grid::Pane) -> Container<'a, Message> {
        let size = 20;

        if PID.access().is_none() {
            return program_message("Start the program to display the terminal.");
        }

        if STDIO.access().is_none() {
            return program_message("Terminal is set to external.");
        }

        let input = text_input("Input...", &self.input)
        .size(size-5).line_height(iced::Pixels(size as f32))
        .on_input(move |text| Message::Pane(PaneMessage::TerminalType(id, text)))
        .on_paste(move |text| Message::Pane(PaneMessage::TerminalPaste(id, text)))
        .on_submit(Message::Pane(PaneMessage::TerminalSend(id)));

        let output = container(
            scrollable(
                text(format!("{}_", state.internal.pane.output)).size(size-5).line_height(0.95) // displaying text and cursor
            ).direction(scrollable::Direction::Both { vertical: scrollbar(), horizontal: scrollbar() })
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
}

#[derive(Debug, Clone, Default)]
pub struct PaneStack {
    open: Vec<bool>,
    unique: u32 // id of the last update (in order to reload the open vec)
}
impl PaneStack {
    fn view<'a>(&self, state: &'a State, id: pane_grid::Pane) -> Container<'a, Message> {
        let stack = match &state.internal.pane.stack {
            Some(stack) => stack,
            None => if PID.access().is_some() {
                return program_message("Stack data not loaded")
            } else {
                return program_message("Start the program to display stack data.")
            }
        };

        if self.unique != state.internal.pane.unique_stack { // if the uniques dont match up, we have old data (we display button to update)
            return container(column![
                text("Old Stack Data").width(Length::Fill).center(),
                container(button(text("Update Stack")).on_press(Message::Pane(PaneMessage::StackUpdate(id)))).width(Length::Fill).center_x(Length::Fill)
            ]).center(Length::Fill).width(Length::Fill).height(Length::Fill)
        };

        let size: u16 = 23;

        let open_vec = &self.open; // sets which lines are displayed

        let mut collapse = column![].width(size);
        let mut lines = column![];

        for (i, open) in open_vec.iter().enumerate() {
            if !open {continue;} // skipping the hidden ones
            let (depth, line) = &stack[i];
            let data = if *depth == 0 { // funtion lines
                text(line).style(style::widget_text)
            } else {
                text(line)
            }.height(size).size(size-5);
            lines = lines.push(
                container(data)
                .padding(padding::left(size*depth.checked_sub(1).unwrap_or(0) as u16)) // removing the indent on the closing brackets of params, while keeping correct collapse rules
            );
            match stack.get(i+1) {
                Some((next_depth, _)) => if next_depth > depth { // if next line has greater indent, we generate a collapse button
                    collapse = collapse.push(Self::collapse_button(open_vec[i+1], i, size, id));
                } else {
                    collapse = collapse.push(container("").height(size));
                }
                None => ()
            };
        }

        let content = container(
            scrollable(
                row![collapse, lines].padding(padding::Padding {bottom: 10., right: 10., ..Default::default()}) // padding for the scrollbars
            ).direction(scrollable::Direction::Both { vertical: scrollbar(), horizontal: scrollbar() })
            .width(Length::Fill)
            .height(Length::Fill)
        ).style(style::back);
        content
    }

    fn collapse_button<'a>(open: bool, index: usize, size: u16, id: pane_grid::Pane) -> button::Button<'a, Message> { // if open, then we create the close one, and vice versa
        if open {
            svg_button("icons/collapse.svg", size, Some(style::collapse_svg))
            .on_press(Message::Pane(PaneMessage::StackCollapse(id, index)))
        } else {
            svg_button("icons/pane_terminal.svg", size, Some(style::collapse_svg_toggled))
            .on_press(Message::Pane(PaneMessage::StackExpand(id, index)))
        }.style(style::breakpoint)
    }
}

#[derive(Debug, Clone)]
pub struct PaneAssembly {
    scrollable: scrollable::Id
}
impl PaneAssembly {
    fn view<'a>(&self, state: &'a State, _id: pane_grid::Pane) -> Container<'a, Message> {
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
                        text(format!("0x{:06x}", address)).style(style::line).font(BOLD) //highlight if its the current rip
                    } else {
                        text(format!("0x{:06x}", address)).style(style::weak)
                    }.size(size-12).center().height(size-5).into()
                )
            );

            let bytes = text(&assembly.bytes) // displaying the bytes
            .size(size-12).line_height(iced::Pixels((size-5) as f32));

            let instructions = text(&assembly.text) // displaying the disassembled instructions
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
            ).direction(scrollable::Direction::Both { vertical: scrollbar(), horizontal: scrollbar() })
            .id(self.scrollable.clone())
            .height(Length::Fill)
            .width(Length::Fill)
        } else {
            return program_message("Assembly not loaded.")
        };
        container(
            assembly
        ).style(style::back)
    }
}
impl Default for PaneAssembly {
    fn default() -> Self {
        Self { scrollable: scrollable::Id::unique() }
    }
}


#[derive(Debug, Clone)]
pub enum LayoutMessage { // Messages regarding the PaneGrid
    SidebarLeftToggle,
    SidebarRightToggle,
    PanelToggle,
    _Focus(pane_grid::Pane),
    Drag(pane_grid::DragEvent),
    Resize(pane_grid::ResizeEvent),
}

#[derive(Debug, Clone)]
pub enum PaneMessage { // Messages regarding the Panes themselves
    // Control
    ControlSelectSignal(pane_grid::Pane, Signal),
    // Registers
    RegistersChangeFormat(pane_grid::Pane, Base),
    // Memory
    MemoryChangeFormat(pane_grid::Pane, ByteBase),
    MemoryToggleSize(pane_grid::Pane),
    MemoryInput(pane_grid::Pane, String),
    MemorySubmit(pane_grid::Pane),
    MemoryPaste(pane_grid::Pane, String),
    MemoryAddress(pane_grid::Pane, iced::mouse::ScrollDelta, i8), // the i8 is as a signed multiplier (eg. scroll by how much per scroll)
    MemoryReset(pane_grid::Pane),
    // Code
    CodeSelectDir(pane_grid::Pane, String),
    CodeSelectFile(pane_grid::Pane, String),
    CodeLoad(Option<pane_grid::Pane>, SourceIndex, String),
    CodeBreakpoints(pane_grid::Pane, Vec<Option<u64>>),
    CodeToggleUpdate(pane_grid::Pane),
    CodeScroll(pane_grid::Pane, scrollable::Viewport),
    // Terminal
    TerminalType(pane_grid::Pane, String),
    TerminalPaste(pane_grid::Pane, String),
    TerminalSend(pane_grid::Pane),
    // Stack
    StackUpdate(pane_grid::Pane),
    StackCollapse(pane_grid::Pane, usize),
    StackExpand(pane_grid::Pane, usize),
    // Assembly
    AssemblyUpdate(Result<(crate::dwarf::Assembly, usize), ()>),
}

// Contents

pub fn content(state: &State) -> Container<'_, Message> { // graphics of the entire UI
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

    fn buttons<'a>(state: &State, size: u16) -> [button::Button<'a, Message>; 4] { // just a wrapper for the buttons
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
    fn status_text<'a>(string: String, mut content: Row<'a, Message>, size: u16, style: Option<impl Fn(&Theme) -> text::Style + 'a>) -> Row<'a, Message> { // appending the text of the status bar
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

    content = match FILE.access().as_ref() { // File name
        None => status_text("File not loaded.".to_string(), content, size, default),
        Some(file) => status_text(format!("File: {}", file.file_name().unwrap().to_str().unwrap()), content, size, Some(style::widget_text)),
    };
    content = content.push(delimiter(10));

    content = match PID.access().as_ref() { // pid
        None => status_text("Program not running".to_string(), content, size, default),
        Some(pid) => status_text(format!("Pid: {}", pid), content, size, Some(style::widget_text)),
    };

    if PID.access().is_some() { // if pid, display the current state of the tracee
        content = content.push(delimiter(10));

        if state.internal.stopped {
            let mut msg = match state.status.unwrap() {
                nix::sys::wait::WaitStatus::Signaled(_, signal, _) => format!("Stopped: {signal}"),
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
        Some(nix::sys::wait::WaitStatus::Exited(pid, ecode)) => { // if no pid but exited status, display exit code
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

// MainFrame

fn main_frame<'a>(state: &'a State) -> Container<'a, Message> {
    container( // creating the pane_grid for the main_frame
        pane_grid(
            &state.layout.panes,
            |id, pane, _maximized| pane_view(id, pane, state)
        ).spacing(10)
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

fn pane_view<'a>(id: pane_grid::Pane, pane: &'a Pane, state: &'a State) -> pane_grid::Content<'a, Message> { // selecting the view function of each pane
    let (content, titlebar) = match pane {
        Pane::Control(control) => (control.view(state, id), pane_titlebar("Control", "icons/pane_control.svg")),
        Pane::Registers(registers) => (registers.view(id), pane_titlebar("Registers", "icons/pane_registers.svg")),
        Pane::Memory(memory) => (memory.view(id), pane_titlebar("Memory", "icons/pane_memory.svg")),
        Pane::Code(code) => (code.view(state, id), pane_titlebar("Code", "icons/pane_source.svg")),
        Pane::Info => (PaneInfo::view(), pane_titlebar("ELF Info", "icons/pane_info.svg")),
        Pane::Terminal(terminal) => (terminal.view(state, id), pane_titlebar("Terminal", "icons/pane_terminal.svg")),
        Pane::Stack(stack) => (stack.view(state, id), pane_titlebar("CallStack", "icons/pane_stack.svg")),
        Pane::Assembly(assembly) => (assembly.view(state, id), pane_titlebar("Assembly", "icons/pane_assembly.svg")),

        _ => (container(text("Some other pane")), pane_grid::TitleBar::new(text("UNDEFINED")))
    };

    pane_grid::Content::new(content).title_bar(titlebar)
}

fn pane_titlebar<'a>(title: &'a str, icon: &'a str) -> pane_grid::TitleBar<'a, Message> { // pane title bar
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

// Code Scrolling

pub fn code_panes_update(state: &mut State) -> Option<(Task<Message>, Task<Message>)> {
    let file = match &state.internal.pane.file {
        Some(file) => file,
        None => return None
    };

    let mut scroll_tasks = Vec::new();
    let mut load_tasks = Vec::new();

    let panes = &mut state.layout.panes;
    for (id, pane) in panes.iter_mut() { // for each code pane that has update set to true, calculate the task to move to the current line and file
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

    Some(( //2 batches of tasks
        Task::batch(scroll_tasks),
        Task::batch(load_tasks)
    ))
}

fn code_update(id: pane_grid::Pane, file: &SourceIndex, pane: &mut PaneCode) -> (Task<Message>, Task<Message>) {
    let size = 25;
    // we find the new dir and new file
    let new_dir = Some(file.hash_path.to_str().unwrap().to_string());
    let file_name = SOURCE.access().as_ref().unwrap().index_with_line(file).path.clone().to_str().unwrap().to_string();

    // we calculate the pixel offset of the line we are stopped at (-3 gives us 3 lines from the top)
    let offset = ((file.line as i32 - 3) * size).max(0);
    let scroll = scrollable::AbsoluteOffset {x: 0., y: offset as f32};

    if new_dir == pane.dir && Some(file_name.clone()) == pane.file {
        let view = match pane.viewport {
            Some(view) => view,
            None => return (scrollable::scroll_to(pane.scrollable.clone(), scroll), Task::none()) // if no viewport, we always scroll
        };
        // we get our current view
        let start = view.absolute_offset().y;
        let end = (view.bounds().height - 6.*size as f32).max(0.); // -6 gives us 3 lines from the bottom (because start is at 3 lines from the top)
        let range = start..start+end;

        if range.contains(&(offset as f32)) { // we scroll only if outside
            return (Task::none(), Task::none());
        };
        if offset as f32 > range.end { // if above the line, we scroll to fit the downwards side, otherwise we scroll to align the highlited line as 3 lines from the top
            let scroll = scrollable::AbsoluteOffset { x: 0., y: offset as f32 - end};
            (scrollable::scroll_to(pane.scrollable.clone(), scroll), Task::none())
        } else {
            (scrollable::scroll_to(pane.scrollable.clone(), scroll), Task::none())
        }
    } else { // if new dir or file, then we select and scroll
        pane.dir = new_dir;
        (scrollable::scroll_to(pane.scrollable.clone(), scroll), Task::done(Message::Pane(PaneMessage::CodeSelectFile(id, file_name))))
    }
}

pub fn check_for_code(state: &mut State) -> bool { // we check if we have any code panes (for performance)
    for (_, pane) in state.layout.panes.iter() {
        match pane {
            Pane::Code(_) => return true,
            _ => ()
        }
    }
    false
}

// Assembly Scrolling

pub fn assembly_scroll(state: &mut State, line: usize, task: &mut Option<Task<Message>>) {
    let panes = &state.layout.panes;
    let offset = scrollable::AbsoluteOffset { x: 0., y: (25.*(line as f32 - 3.)).max(0.)}; // we scroll to the middle -3 lines as the highlight is always in the middle

    for (_, pane) in panes.iter() { // for each assembly pane
        match pane {
            Pane::Assembly(data) => {
                *task = Some(scrollable::scroll_to(data.scrollable.clone(), offset))
            }
            _ => ()
        }
    };
}

pub fn check_for_assembly(state: &mut State) -> bool { // we check if we have any assembly panes (for performance)
    for (_, pane) in state.layout.panes.iter() {
        match pane {
            Pane::Assembly(_) => return true,
            _ => ()
        }
    }
    false
}

// PaneMessage Handle (Mainframe Operations)

fn get_pane<'a>(panes: &'a mut pane_grid::State<Pane>, pane: pane_grid::Pane) -> &'a mut Pane { //pane wrapper
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
        PaneMessage::CodeSelectDir(pane, dir) => { // setting the directory and reseting the file (unless the same one has been selected)
            let data = get_pane(panes, pane).code();
            data.viewport = None;
            if data.dir == Some(dir.clone()) {
                return;
            }
            data.dir = Some(dir);
            data.file = None;
        },
        PaneMessage::CodeSelectFile(pane, file) => { // setting the file and creating the breakpoints task
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
            if code.0.content.is_some() { //Conditional file load (if we dont have the contents)
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
        PaneMessage::CodeLoad(pane, index, text) => { // if file contents loaded, create also the breakpoints, if the id of the pane is provided
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
        PaneMessage::CodeBreakpoints(pane, breakpoints) => { // sets the breakpoints
            get_pane(panes, pane).code().breakpoints = breakpoints;
        },
        PaneMessage::CodeToggleUpdate(pane) => { // toggle the update bool, if newly true, perform update right away to find the highlight line
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
        PaneMessage::MemorySubmit(pane) => { // we try converting both, choosing the one that is correct, prefering dec (because hex contains dec)
            let data = get_pane(panes, pane).memory();
            let field = &data.field;
            let hex = u64::from_str_radix(field.get(2..).unwrap_or("g"), 16); // g to produce a convesion error
            let dec = u64::from_str_radix(field, 10);
            if hex.is_err() && dec.is_err() {
                data.incorrect = true; // if not a number
                return;
            };
            let num = match dec {
                Ok(num) => num,
                Err(_) => hex.unwrap()
            };
            data.address = num;
            data.incorrect = false; // reset the NaN error
            update_memory(data); // we trigger update memory check
        },
        PaneMessage::MemoryAddress(pane, delta, mult) => {
            let data = get_pane(panes, pane).memory();
            let y = match delta {
                iced::mouse::ScrollDelta::Lines { x: _, y } => y*mult as f32,
                iced::mouse::ScrollDelta::Pixels { x: _, y } => y*mult as f32,
            };
            if data.more_bytes { // limiting the address in u64 bounds AND if outside of the alignment, then first align to the closest, then continue normally
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
            } else { // 8 and 4 byte alignment
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
            let hex_check = data.field.get(0..2); // we check for hexadecimal, to keep the previous format
            if hex_check.is_none() || hex_check.unwrap() == "0x" {
                data.field = format!("0x{:x}",data.address);
            } else {
                data.field = format!("{}",data.address);
            }
            data.incorrect = false; // reset the NaN error
            update_memory(data);
        },
        PaneMessage::MemoryReset(pane) => {
            let data = get_pane(panes, pane).memory(); // we get the beginning of the memory (from tge memory maps)
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
            if hex_check.is_none() || hex_check.unwrap() == "0x" { // check for hexadecimal to keep the previous format
                data.field = format!("0x{:x}",data.address);
            } else {
                data.field = format!("{}",data.address);
            }
            data.incorrect = false; // reset the NaN error
            update_memory(data); // we trigger update memory
        },
        // Terminal
        PaneMessage::TerminalType(pane, data) => get_pane(panes, pane).terminal().input = data,
        PaneMessage::TerminalPaste(pane, data) => get_pane(panes, pane).terminal().input = data,
        PaneMessage::TerminalSend(pane) => {
            let data = get_pane(panes, pane).terminal();
            data.input.push('\n');

            if object::stdio().unwrap().write(data.input.as_bytes()).is_err() { // we write to the PTY with a newline attached
                return;
            };
            data.input.clear(); // and clear the input
        },
        // Assembly
        PaneMessage::AssemblyUpdate(result) => { // setting the newly produced assembly, and scrolling the panes
            match result {
                Ok((assembly, line)) => {
                    state.internal.pane.assembly = Some(assembly);
                    assembly_scroll(state, line, task);
                }
                Err(()) => state.internal.pane.assembly = None
            }
        }
        // Stack
        PaneMessage::StackUpdate(pane) => { // creating the open vec from the new stack data, and setting the unique to be the same as the global
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

fn stack_open(stack: &Vec<(usize, String)>, pane: &mut PaneStack, line: usize, open: bool) { // we expand or collapse the lines until we get to the same level again
    let upper = stack[line].0;
    let open_vec = &mut pane.open;
    for (i, (depth, _)) in stack.iter().skip(line+1).enumerate() { // skipping the first lines
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
                _ => ()
            };
        }
        LayoutMessage::Resize(pane_grid::ResizeEvent {split, ratio}) => {
            resize(&mut state.layout, split, ratio);
        }
        _ => ()
    };
}

fn layout(layout: &mut Layout, pane: LayoutMessage) { // layout update
    let (main, left, right, panel) = layout.get_nodes(); // gets the parts
    let mut saved_state = SAVED_STATE.access().clone().unwrap(); 

    saved_state.main = Some(layout.node_to_configuration(&main));
    match left {// updates the SAVED STATE as we are getting rid of some panes
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
            if layout.panel_mode == config::PanelMode::left { // if panel mode left and we toggle the panel, we need to recalculate the ratios (as the configuration is getting changed)
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

    SAVED_STATE.sets(saved_state.clone()); // we set the update parts

    let base = Layout::base(layout.sidebar_left, layout.sidebar_right, layout.panel, &layout.panel_mode, saved_state); // create a new base from the parts

    layout.panes = pane_grid::State::with_configuration(base); // and finally set the new state from the configuration
}

fn resize(layout: &mut Layout, split: pane_grid::Split, ratio: f32) { // big resize logic function (mainly because of sidebars)
    if layout.panel_mode == config::PanelMode::left && layout.panel { // opposite rules (because of the configuration)
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

    if layout.sidebar_right { // if right sidebar is present, then we just use the resize function, otherwise we resize the saved state
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


// Formating Helpers
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
pub enum ByteBase { // like base but for single bytes
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

// Widgets helpers

fn scrollbar() -> scrollable::Scrollbar {
    scrollable::Scrollbar::new().scroller_width(0).width(0)
}

fn no_scrollbar() -> scrollable::Scrollbar {
    scrollable::Scrollbar::new().scroller_width(0).width(0)
}

fn svg_button<'a>(icon: &str, size: u16, svg_style: Option<fn(&Theme, svg::Status) -> svg::Style>) -> button::Button<'a, Message> {
    button(
        svg(Handle::from_memory(Asset::get(icon).unwrap().data))
        .height(Length::Fill)
        .style(svg_style.unwrap_or(style::bar_svg))
    ).padding(4)
    .height(size)
    .width(size)
}

fn breakpoint_button<'a>(address: Option<u64>, size: u16) -> button::Button<'a, Message> { // if address in the breakpoints, then toggled
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