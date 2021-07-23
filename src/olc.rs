#![allow(non_snake_case)]
#![allow(deprecated)]
#![allow(clippy::too_many_arguments)]
//#![feature(nll)]

use crate::engine::OLCEngine;

#[derive(Debug)]
pub enum Rcode {
    Fail,
    Ok,
    NoFile,
}

pub type OlcFuture<T> = std::pin::Pin<Box<dyn std::future::Future<Output = T>>>;

pub trait Olc<D: 'static + OlcData> {
    fn on_engine_start(&self, engine: OLCEngine<D>) -> Result<OlcFuture<OLCEngine<D>>, &str>;

    fn on_engine_update(&self, engine: &mut OLCEngine<D>, elapsedTime: f64) -> Result<(), &str>;

    fn on_engine_destroy(&self, engine: &mut OLCEngine<D>) -> Result<(), &str>;
}

pub trait OlcData {}
