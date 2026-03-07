use iced::{
    Task, Length,
    widget::{
        Container, Row, Theme,
        button, column, container, mouse_area, pane_grid, row, svg, text,
        svg::Handle
    },
    padding
};

use crate::{
    window::*, data::*, trace::*, style, config
};

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

#[derive(Debug, Clone, Default)]
struct PaneMemory {
    address: u64, // where are we in memory, we read extra 1KB around this area and store to global data, and update only when we get outside of this region, for read effectivity
    bytes_per_row: u8, //min 4, max 16
    binary_display: bool,
    read_error: bool, // if read error occurs, show a button to take the user back (resets the address to a correct map)
    _selected: Select, // TODO feature
    //_colored: HashMap<MemColor, Select>, // TODO feature
}

#[derive(Debug, Clone, Default)]
struct Select {
    address: u64,
    range: Option<u32> // option only for writing purposes
}

//#[derive(Debug, Clone)]
//enum MemColor {} // TODO feature

#[derive(Debug, Clone, Default)]
struct PaneStack {} // TODO

#[derive(Debug, Clone, Default)]
struct PaneCode {}

#[derive(Debug, Clone, Default)]
struct PaneAssembly {} // TODO

#[derive(Debug, Clone, Default)]
struct PaneRegisters {} //TODO

#[derive(Debug, Clone, Default)]
struct PaneInfo {} // TODO

#[derive(Debug, Clone, Default)]
struct PaneControl {} // TODO

#[derive(Debug, Clone, Default)]
struct PaneTerminal {
    text: String
}

#[derive(Debug, Clone)]
pub enum PaneMessage {
    SidebarLeftToggle,
    SidebarRightToggle,
    PanelToggle,
    _Focus(pane_grid::Pane),
    Drag(pane_grid::DragEvent),
    Resize(pane_grid::ResizeEvent),
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
        .on_press(Message::Pane(PaneMessage::SidebarLeftToggle));

        let sidebar_right = svg_button("icons/sidebar_right.svg", size,
        Some(if state.layout.sidebar_right {style::bar_svg_toggled} else {style::bar_svg})
        ).style(if state.layout.sidebar_right {style::bar_button_toggled} else {style::bar_button})
        .on_press(Message::Pane(PaneMessage::SidebarRightToggle));

        let panel = svg_button("icons/panel.svg", size,
        Some(if state.layout.panel {style::bar_svg_toggled} else {style::bar_svg})
        ).style(if state.layout.panel {style::bar_button_toggled} else {style::bar_button})
        .on_press(Message::Pane(PaneMessage::PanelToggle));

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
        .on_click(|pane| Message::Pane(PaneMessage::_Focus(pane)))
        .on_drag(|drag_event| Message::Pane(PaneMessage::Drag(drag_event)))
        .on_resize(10, |resize_event| Message::Pane(PaneMessage::Resize(resize_event)))
        .spacing(2)
    ).center(Length::Fill)
    .width(Length::Fill)
    .height(Length::Fill)
}

fn pane_view<'a>(id: pane_grid::Pane, pane: &Pane, state: &'a State) -> pane_grid::Content<'a, Message> {
    let (content, titlebar) = match pane {
        Pane::Code(state) => (pane_view_code(state), pane_titlebar("Code", "icons/pane_source.svg")),
        Pane::Control(state) => (pane_view_control(state), pane_titlebar("Control", "icons/pane_control.svg")),
        Pane::Memory(state) => (pane_view_memory(state), pane_titlebar("Memory", "icons/pane_memory.svg")),
        Pane::Stack(state) => (pane_view_stack(state), pane_titlebar("CallStack", "icons/pane_stack.svg")),
        Pane::Registers(state) => (pane_view_registers(state), pane_titlebar("Registers", "icons/pane_registers.svg")),
        Pane::Assembly(state) => (pane_view_assembly(state), pane_titlebar("Assembly", "icons/pane_assembly.svg")),
        Pane::Terminal(state) => (pane_view_terminal(state), pane_titlebar("Terminal", "icons/pane_terminal.svg")),
        Pane::Info(state) => (pane_view_info(state), pane_titlebar("ELF Info", "icons/pane_info.svg")),

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

fn pane_view_code<'a>(state: &PaneCode) -> Container<'a, Message> {
    let content = container(text("CODE"));
    content
}

fn pane_view_control<'a>(state: &PaneControl) -> Container<'a, Message> {
    let size = 30;

    let file = FILE.access().is_some();
    let running = PID.access().is_some();

    let start_stop = if running {
        svg_button("icons/stop.svg", size, Some(style::widget_svg))
        .style(style::widget_button)
        .on_press(Message::Operation(Operation::StopTracee))
    } else {
        svg_button("icons/run.svg", size, Some(style::widget_svg))
        .style(style::widget_button)
        .on_press(Message::Operation(Operation::RunTracee))
    };

    let pause = svg_button("icons/pause.svg", size, Some(style::bar_svg))
    .on_press_maybe(if running {Some(Message::Operation(Operation::StopTracee))} else {None})
    .style(style::widget_button);

    let step = svg_button("icons/pause.svg", size, Some(style::bar_svg))
    .on_press_maybe(if running {Some(Message::Operation(Operation::StopTracee))} else {None})
    .style(style::widget_button);

    //let cont = ;

    //let signal = ;

    let content = container(row![
        start_stop,
        pause
    ]);
    content
}

fn pane_view_memory<'a>(state: &PaneMemory) -> Container<'a, Message> {
    let content = container(text("TERMINAL"));
    content
}

fn pane_view_stack<'a>(state: &PaneStack) -> Container<'a, Message> {
    let content = container(text("TERMINAL"));
    content
}

fn pane_view_registers<'a>(state: &PaneRegisters) -> Container<'a, Message> {
    let content = container(text("TERMINAL"));
    content
}

fn pane_view_assembly<'a>(state: &PaneAssembly) -> Container<'a, Message> {
    let content = container(text("TERMINAL"));
    content
}

fn pane_view_terminal<'a>(state: &PaneTerminal) -> Container<'a, Message> {
    let content = container(text("TERMINAL"));
    content
}

fn pane_view_info<'a>(state: &PaneInfo) -> Container<'a, Message> {
    let content = container(text("INFO"));
    content
}




//message Handle

pub fn pane_message(state: &mut State, pane: PaneMessage) {
    match pane {
        PaneMessage::SidebarLeftToggle => layout(&mut state.layout, pane),
        PaneMessage::SidebarRightToggle => layout(&mut state.layout, pane),
        PaneMessage::PanelToggle => layout(&mut state.layout, pane),

        PaneMessage::_Focus(pane) =>   state.layout._focus = Some(pane),
        PaneMessage::Drag(pane_grid::DragEvent::Dropped {pane, target}) => {
            match target {
                pane_grid::Target::Pane(target_pane, _) => state.layout.panes.swap(pane, target_pane),
                //pane_grid::Target::Edge(edge) => state.layout.panes.drop(pane, pane_grid::Target::Edge(edge))
                _ => ()
            };
        }
        PaneMessage::Resize(pane_grid::ResizeEvent {split, ratio}) => {
            //state.layout.panes.resize(split, ratio)
            resize(&mut state.layout, split, ratio);
        }
        //fill in later
        _ => ()
    };
}

fn layout(layout: &mut Layout, pane: PaneMessage) {
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
        PaneMessage::SidebarLeftToggle => layout.sidebar_left ^= true,
        PaneMessage::SidebarRightToggle => layout.sidebar_right ^= true,
        PaneMessage::PanelToggle => {
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

fn limit(ratio: f32) -> f32 { // this is not up for debate, this is to prevent some WEIRD graphics to appear
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

// 