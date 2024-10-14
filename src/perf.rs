
use std::{collections::VecDeque, time::{Duration, Instant}};

use valence::prelude::*;
use bevy::prelude::*;
use bevy_time::TimePlugin;
use tracing::info;

pub struct Perf;

impl Plugin for Perf {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(PrintTimer::new())
            .insert_resource(TickTimer(Instant::now()))
            .add_systems(PreUpdate, performance_pre)
            .add_systems(PostUpdate, performance_post);
    }
}

#[derive(Resource)]
struct PrintTimer(Timer, VecDeque<Duration>);

impl PrintTimer {
    fn new() -> Self {
        Self(
            Timer::new(
                Duration::from_secs(1),
                TimerMode::Repeating,
            ),
            VecDeque::new(),
        )
    }
}

#[derive(Resource)]
struct TickTimer(Instant);

fn performance_pre(mut tick_start: ResMut<TickTimer>) {
    tick_start.0 = Instant::now();
}

fn performance_post(
    time: Res<Time>,
    mut timer: ResMut<PrintTimer>,
    tick_start: Res<TickTimer>,
) {
    let elapsed = Instant::now() - tick_start.0;
    if timer.1.len() >= 20 {
        timer.1.pop_front();
    }
    timer.1.push_back(elapsed);
    let average = timer.1.iter().fold(Duration::ZERO, |acc, t| acc + *t) / timer.1.len() as u32;
    if timer.0.tick(time.delta()).finished() {
        info!("tick time: {:?}", average);
    }
}
