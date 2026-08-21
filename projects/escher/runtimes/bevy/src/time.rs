use core::ops::Deref;
use core::ops::DerefMut;
use core::time::Duration;

use bevy::ecs::component::Component;
use bevy::ecs::message::Message;
use bevy::prelude::MessageWriter;
use bevy::prelude::Query;
use bevy::prelude::Res;
use bevy::time::Time;
use bevy::time::Timer;
use bevy::time::TimerMode;

#[derive(Component)]
pub struct DrawTimer(Timer);

impl DrawTimer {
    pub fn new(duration: Duration) -> Self {
        let mut timer = Timer::new(duration, TimerMode::Repeating);
        timer.set_elapsed(duration);
        DrawTimer(timer)
    }
}

impl Deref for DrawTimer {
    type Target = Timer;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DrawTimer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Fires each time a `DrawTimer` completes a cycle. The inner `Duration` is the time elapsed
/// since the app started, not since the timer last reset.
#[derive(Message, Default, Debug)]
pub struct DrawTimerFinishedEvent(Duration);

impl DrawTimerFinishedEvent {
    pub fn new(duration: Duration) -> Self {
        DrawTimerFinishedEvent(duration)
    }

    pub fn duration(&self) -> Duration {
        self.0
    }
}

pub fn sync_draw_timer(
    mut timer_query: Query<&mut DrawTimer>,
    mut timer_finished_evtw: MessageWriter<DrawTimerFinishedEvent>,
    time: Res<Time>,
) {
    for mut timer in timer_query.iter_mut() {
        if timer.tick(time.delta()).just_finished() {
            timer_finished_evtw.write(DrawTimerFinishedEvent(time.elapsed()));
        }
    }
}
