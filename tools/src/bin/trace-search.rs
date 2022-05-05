#![feature(is_some_with)]

use std::fmt::Debug;
use std::sync::atomic::AtomicBool;
use std::{env, process};

use kingly_lib::eval::StandardEval;
use kingly_lib::move_gen::MoveGen;
use kingly_lib::position::Position;
use kingly_lib::search::{Search, TranspositionTable};
use kingly_lib::tables::Tables;
use tracing::field::{Field, Visit};
use tracing::span::Attributes;
use tracing::subscriber::set_global_default;
use tracing::{Event, Id, Level, Metadata, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{prelude::*, registry, Layer};
use valuable::Value;

struct Tree;

impl Tree {}

impl Visit for Tree {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        dbg!(field, value);
    }

    fn record_value(&mut self, field: &Field, value: Value) {
        dbg!(field, value);
    }
}

struct SearchLayer;

impl<S: Subscriber> Layer<S> for SearchLayer
where
    for<'lookup> S: LookupSpan<'lookup>,
{
    fn enabled(&self, metadata: &Metadata, _ctx: Context<'_, S>) -> bool {
        metadata.target() == "kingly_lib::search" && *metadata.level() == Level::TRACE
    }

    fn on_new_span(&self, attrs: &Attributes, _id: &Id, _ctx: Context<'_, S>) {
        //dbg!(attrs.values());
    }

    fn on_event(&self, event: &Event, ctx: Context<'_, S>) {
        if let Some(span) = ctx.current_span().metadata() {
            match span.name() {
                "search" => {
                    event.record(&mut Tree);
                }
                "aspiration" => {}
                _ => {}
            }
        } else {
            panic!()
        }
    }
}

fn main() {
    let args: Vec<_> = env::args().collect();

    if args.len() != 3 {
        println!("Usage: trace-search fen depth");
        process::exit(1);
    }

    let subscriber = registry().with(SearchLayer);
    set_global_default(subscriber).expect("setting default subscriber failed");

    let position = Position::from_fen(&args[1]).unwrap();
    let move_gen = MoveGen::new(Tables::get());
    let eval = StandardEval::new(Tables::get());
    let mut trans_table = TranspositionTable::new();
    let depth: u8 = args[2].parse().unwrap();

    Search::new(position, move_gen, eval, &mut trans_table)
        .depth(depth)
        .start(&AtomicBool::new(false));
}
