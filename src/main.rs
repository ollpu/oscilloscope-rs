use tuix::*;
use std::{result::Result, error::Error};
use std::vec::Vec;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::convert::TryInto;

static THEME: &'static str = include_str!("theme.css");

fn main() -> Result<(), Box<dyn Error>> {
    let host = cpal::default_host();
    let device = host.default_input_device().expect("failed to find input device");
    eprintln!("Using input device: \"{}\"", device.name()?);
    let mut config: cpal::StreamConfig = device.default_input_config()?.into();
    config.channels = 1;
    let (mut plot, mut plot_ingest) = Plot::new_and_ingestor(config.sample_rate.0);
    let audio_cb = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        plot_ingest.process(data);
    };
    let input_stream = device.build_input_stream(&config, audio_cb, err_fn)?;
    input_stream.play()?;
    let mut app = Application::new(move |win_desc, state, window| {
        state.insert_theme(THEME);
        plot.build(state, window, |builder| builder.set_flex_grow(1.0));
        win_desc.with_title("Oscilloscope").with_inner_size(800, 600)
    });
    app.run();
    Ok(())
}
fn err_fn(err: cpal::StreamError) {
    eprintln!("an error occurred on stream: {}", err);
}

struct PlotIngest {
    publish_handle: triple_buffer::Input<[f32; 512]>,
    buffer: Vec<f32>,
    interval: u32,
    clock: u32,
}

impl PlotIngest {
    fn process(&mut self, data: &[f32]) {
        for sample in data {
            if self.clock < 512 {
                self.buffer.push(*sample);
                if self.buffer.len() == 512 {
                    if let Ok(array) = self.buffer[..].try_into() {
                        self.publish_handle.write(array);
                    }
                    self.buffer.clear();
                }
            }
            self.clock += 1;
            if self.clock == self.interval {
                self.clock = 0;
            }
        }
    }
}

struct Plot {
    consume_handle: triple_buffer::Output<[f32; 512]>,
    last: std::time::Instant,
}

impl Plot {
    pub fn new_and_receiver(sample_rate: u32) -> (Self, PlotIngest) {
        let buffer = triple_buffer::TripleBuffer::new([0.; 512]);
        let (buf_in, buf_out) = buffer.split();
        (
            Plot { consume_handle: buf_out, last: std::time::Instant::now() },
            PlotIngest {
                publish_handle: buf_in,
                buffer: Vec::with_capacity(512),
                interval: std::cmp::max(sample_rate / 60, 512),
                clock: 0,
            }
        )
    }
}

impl BuildHandler for Plot {
    type Ret = Entity;
    fn on_build(&mut self, state: &mut State, entity: Entity) -> Self::Ret {
        state.style.insert_element(entity, "plot");
        entity
    }
}

use femtovg::{
    renderer::OpenGl, Baseline, Canvas, Color, FillRule, FontId, ImageFlags, ImageId, LineCap,
    LineJoin, Paint, Path, Renderer, Solidity,
};

impl EventHandler for Plot {
    fn on_draw(&mut self, state: &mut State, entity: Entity, canvas: &mut Canvas<OpenGl>) {
        // XXX: FPS print
        // println!("{}", 1./(self.last.elapsed().as_secs_f32()));
        self.last = std::time::Instant::now();
        std::thread::sleep_ms(15);
        state.insert_event(Event::new(WindowEvent::Redraw));
        let mut path = Path::new();
        let buf = self.consume_handle.read();
        let mut points = buf.iter().enumerate().map(|(i, v)| {
            (i as f32, 200.-v*200.)
        });
        let fst = points.next().unwrap();
        path.move_to(fst.0, fst.1);
        for p in points {
            path.line_to(p.0, p.1);
        }
        canvas.stroke_path(&mut path, Paint::color(Color::rgb(255, 0, 0)));
        
    }
}
