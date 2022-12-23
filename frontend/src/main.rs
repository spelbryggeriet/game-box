#![allow(non_snake_case)]

use std::f32::consts::PI;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::RwLock;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use dioxus::{core::to_owned, prelude::*};
use fermi::prelude::*;
use rand::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::Window;

use model::{CellGrid, CellShape};

mod model;

const BOARD_VIEW_SIZE: usize = 75;

fn sleep(timeout_ms: i32) -> SetTimeout {
    let finished = Arc::new(AtomicBool::new(false));
    let finished_clone = Arc::clone(&finished);
    let waker = Arc::new(RwLock::new(Option::<Waker>::None));
    let waker_clone = Arc::clone(&waker);

    let c = Closure::<dyn Fn()>::new(move || {
        finished_clone.store(true, Ordering::SeqCst);
        waker_clone
            .read()
            .expect("thread should not be poisoned")
            .expect("waker should be set")
            .wake_by_ref()
    });

    web_sys::window()
        .expect("no global `window` exists")
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            c.as_ref().unchecked_ref(),
            timeout_ms,
        )
        .expect("should register `setTimeout` OK");
    c.forget();

    SetTimeout { finished, waker }
}

struct SetTimeout {
    finished: Arc<AtomicBool>,
    waker: Arc<RwLock<Option<Waker>>>,
}

impl Future for SetTimeout {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let fut = Pin::into_inner(self);
        if fut.finished.load(Ordering::SeqCst) {
            Poll::Ready(())
        } else {
            let waker = cx.waker().clone();
            fut.waker
                .write()
                .expect("thread should not be poisened")
                .replace(waker);
            Poll::Pending
        }
    }
}

#[inline_props]
fn Board<'a>(
    cx: Scope<'a>,
    cell_grid: &'a UseState<CellGrid>,
    turn: &'a UseState<Option<CellShape>>,
) -> Element {
    let size = cell_grid.size();
    let initialized = use_state(&cx, || false);

    let elem = cx.render(rsx! {
        div {
            class: "board",
            div {
                class: "cells",
                style: "grid-template: repeat({size}, 1fr) / repeat({size}, 1fr)",
                (0..size).flat_map(|i| {
                    (0..size).map(move |j| {
                        rsx! {
                            Cell {
                                row: i,
                                col: j,
                                cell_grid: (*cell_grid).clone(),
                                turn: (*turn).clone(),
                            }
                        }
                    })
                }),
            },
            (1..size).map(|i| {
                rsx! {
                    Line {
                        start_left: "0px",
                        start_top: "calc({i} * min({BOARD_VIEW_SIZE}vh, {BOARD_VIEW_SIZE}vw)
                                    / {size})",
                        length: "100%",
                        thickness: "5px",
                        random_delay: true,
                        hidden: !**initialized,
                    }
                }
            }),
            (1..size).map(|j| {
                rsx! {
                    Line {
                        start_left: "calc({j} * min({BOARD_VIEW_SIZE}vh, {BOARD_VIEW_SIZE}vw)
                                     / {size})",
                        start_top: "0px",
                        length: "100%",
                        thickness: "5px",
                        rotation: "90deg",
                        random_delay: true,
                        hidden: !**initialized,
                    }
                }
            }),
        }
    });

    if !**initialized {
        initialized.set(true);
    }

    elem
}

#[inline_props]
fn Cell(
    cx: Scope,
    row: usize,
    col: usize,
    cell_grid: UseState<CellGrid>,
    turn: UseState<Option<CellShape>>,
) -> Element {
    let shape = cell_grid[*row][*col];
    let row_one_indexed = row + 1;
    let col_one_indexed = col + 1;
    let circle_hidden = shape != Some(CellShape::Circle);
    let cross_hidden = shape != Some(CellShape::Cross);

    cx.render(rsx! {
        div {
            class: "cell",
            style: "grid-row: {row_one_indexed};
                    grid-column: {col_one_indexed}",
            onclick: move |_| if let Some(turn_shape) = **turn {
                if shape.is_none() {
                    cell_grid.make_mut()[*row][*col].replace(turn_shape);

                    cx.spawn({
                        to_owned![cell_grid, turn];
                        async move {
                            let current_cell_grid = cell_grid.current();
                            if current_cell_grid.is_solved() {
                                turn.set(None);
                                sleep(500).await;
                                cell_grid.make_mut().clear_non_solved();
                                sleep(750).await;
                                cell_grid.make_mut().clear_all();
                                sleep(750).await;
                                turn.set(Some(!turn_shape));
                            } else if current_cell_grid.is_full() {
                                turn.set(None);
                                sleep(750).await;
                                cell_grid.make_mut().clear_all();
                                sleep(750).await;
                                turn.set(Some(!turn_shape));
                            } else {
                                turn.set(Some(!turn_shape));
                            }
                        }
                    })
                }
            },
            Circle {
                hidden: circle_hidden,
            },
            Cross {
                hidden: cross_hidden,
            },
        }
    })
}

#[inline_props]
fn Circle(cx: Scope, hidden: bool) -> Element {
    let mut rng = thread_rng();
    let duration = rng.gen_range(0.3..0.6);
    let rotation = rng.gen_range(0..360);
    let flip = if rng.gen() { "scaleX(-1)" } else { "" };
    let hidden = if *hidden { "hidden" } else { "" };

    cx.render(rsx! {
        div {
            class: "circle {hidden}",
        }
        svg {
            style: "transform: rotate({rotation}deg) {flip};",
            view_box: "0 0 100 100",
            xmlns: "http://www.w3.org/2000/svg",
            circle {
                style: "transition-duration: {duration}s;",
                r: "24",
            }
        }
    })
}

#[inline_props]
fn Cross(cx: Scope<CrossProps>, hidden: bool) -> Element {
    const THICKNESS: f32 = 15.0;
    const SQRT2_PERCENT: f32 = 141.42136;

    let offset = (PI / 4.0).cos() * THICKNESS / 2.0;

    cx.render(rsx! {
        Line {
            start_left: "{offset:0.2}px",
            start_top: "{offset:0.2}px",
            length: "calc({SQRT2_PERCENT:0.2}% - {THICKNESS:0.2}px)",
            thickness: "{THICKNESS:0.2}px",
            rotation: "45deg",
            hidden: *hidden,
        }
        Line {
            start_left: "{offset:0.2}px",
            start_top: "calc(100% - {offset:0.2}px)",
            length: "calc({SQRT2_PERCENT:0.2}% - {THICKNESS:0.2}px)",
            thickness: "{THICKNESS:0.2}px",
            rotation: "-45deg",
            hidden: *hidden,
        }
    })
}

#[inline_props]
fn Line<'a>(
    cx: Scope,
    start_left: &'a str,
    start_top: &'a str,
    length: &'a str,
    thickness: &'a str,
    rotation: Option<&'a str>,
    #[props(default)] random_delay: bool,
    #[props(default)] hidden: bool,
) -> Element<'a> {
    let mut rng = thread_rng();

    let rotation = rotation.map(|r| format!(" rotate({r})"));
    let scaling = if rng.gen() { Some("scaleX(-1)") } else { None };
    let transform = if rotation.as_deref().or(scaling).is_some() {
        format!(
            "transform:
                translate(-50%)
                {}
                translate(50%)
                {}",
            rotation.unwrap_or_default(),
            scaling.unwrap_or_default(),
        )
    } else {
        String::new()
    };

    let duration = rng.gen_range(0.2..0.5);
    let delay = if *random_delay {
        let delay = rng.gen_range(0.0..0.15);
        format!("transition-delay: {delay:0.2}s;")
    } else {
        String::new()
    };
    let hidden = if *hidden { "hidden" } else { "" };

    cx.render(rsx! {
        div {
            class: "line {hidden}",
            style: "left: {start_left};
                    top: calc(({start_top}) - ({thickness}) / 2);
                    width: {length};
                    height: {thickness};
                    {transform};
                    transition-duration: {duration:0.2}s;
                    {delay};",
        }
    })
}

fn App(cx: Scope) -> Element {
    const SIZE: usize = 3;

    let cell_grid = use_state(&cx, || CellGrid::new(SIZE));
    let turn = use_state(&cx, || Some(CellShape::Circle));

    cx.render(rsx! {
        Board {
            cell_grid: cell_grid,
            turn: turn,
        }
    })
}

fn main() {
    dioxus::web::launch(App);
}
