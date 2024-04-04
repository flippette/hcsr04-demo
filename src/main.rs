//! HC-SR04 demo
//!
//! Vcc -> +5V
//! Trig -> PA2
//! Echo -> PA3
//! GND -> NPN -> LED -> Resistor -> GND

#![no_std]
#![no_main]

use core::pin::pin;

use defmt_rtt as _;
use panic_probe as _;

use defmt::info;
use embassy_executor::{main, task, Spawner};
use embassy_stm32::{
    exti::ExtiInput,
    gpio::{Input, Level, Output, Pull, Speed},
    peripherals::{PA10, PA2, PA3, PC13},
};
use embassy_time::{Instant, Timer};
use futures_util::{select_biased, FutureExt};

#[main]
async fn main(s: Spawner) {
    let p = embassy_stm32::init(<_>::default());

    let trig = Output::new(p.PA3, Level::Low, Speed::High);
    let echo = ExtiInput::new(Input::new(p.PA2, Pull::Down), p.EXTI2);
    let reset = Output::new(p.PA10, Level::High, Speed::Low);
    s.must_spawn(hcsr04(trig, echo, reset));

    let btn = ExtiInput::new(Input::new(p.PC13, Pull::Down), p.EXTI13);
    s.must_spawn(button(btn));
}

#[task]
async fn hcsr04(
    mut trig: Output<'static, PA3>,
    mut echo: ExtiInput<'static, PA2>,
    mut reset: Output<'static, PA10>,
) {
    loop {
        // measurement loop
        let echo_dur = loop {
            // trigger pulses
            trig.set_low();
            Timer::after_micros(5).await;
            trig.set_high();
            Timer::after_micros(10).await;
            trig.set_low();
            info!("triggered pulses!");

            let echo_start = Instant::now();
            let mut echo_wait = pin!(async {
                echo.wait_for_high().await;
                info!("saw high!");
                echo.wait_for_low().await;
                info!("saw low!");
            }
            .fuse());
            let mut timeout = Timer::after_millis(40).fuse();

            select_biased! {
                () = echo_wait => break Instant::now() - echo_start,
                () = timeout => {
                    info!("timeout expired, retrying measurement!");
                    reset.set_low();
                    Timer::after_millis(50).await;
                    reset.set_high();
                    continue;
                },
            }
        };

        let dist = (echo_dur.as_micros() / 2) as f64 * 343e-6;
        info!("dist = {}m", dist);

        Timer::after_secs(1).await;
    }
}

#[task]
async fn button(mut btn: ExtiInput<'static, PC13>) {
    loop {
        btn.wait_for_rising_edge().await;
        info!("pressed!");
        btn.wait_for_falling_edge().await;
        info!("released!");
    }
}
