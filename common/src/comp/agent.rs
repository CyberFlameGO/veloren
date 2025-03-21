use crate::{
    comp::{
        biped_small, bird_medium, humanoid, quadruped_low, quadruped_medium, quadruped_small, ship,
        Body, UtteranceKind,
    },
    path::Chaser,
    rtsim::{Memory, MemoryItem, RtSimController, RtSimEvent},
    trade::{PendingTrade, ReducedInventory, SiteId, SitePrices, TradeId, TradeResult},
    uid::Uid,
};
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage, Entity as EcsEntity};
use specs_idvs::IdvStorage;
use std::{collections::VecDeque, fmt};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use vek::*;

use super::dialogue::Subject;

pub const DEFAULT_INTERACTION_TIME: f32 = 3.0;
pub const TRADE_INTERACTION_TIME: f32 = 300.0;
const AWARENESS_DECREMENT_CONSTANT: f32 = 2.1;
const SECONDS_BEFORE_FORGET_SOUNDS: f64 = 180.0;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Alignment {
    /// Wild animals and gentle giants
    Wild,
    /// Dungeon cultists and bandits
    Enemy,
    /// Friendly folk in villages
    Npc,
    /// Farm animals and pets of villagers
    Tame,
    /// Pets you've tamed with a collar
    Owned(Uid),
    /// Passive objects like training dummies
    Passive,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Mark {
    Merchant,
    Guard,
}

impl Alignment {
    // Always attacks
    pub fn hostile_towards(self, other: Alignment) -> bool {
        match (self, other) {
            (Alignment::Passive, _) => false,
            (_, Alignment::Passive) => false,
            (Alignment::Enemy, Alignment::Enemy) => false,
            (Alignment::Enemy, Alignment::Wild) => false,
            (Alignment::Wild, Alignment::Enemy) => false,
            (Alignment::Wild, Alignment::Wild) => false,
            (Alignment::Npc, Alignment::Wild) => false,
            (Alignment::Npc, Alignment::Enemy) => true,
            (_, Alignment::Enemy) => true,
            (Alignment::Enemy, _) => true,
            _ => false,
        }
    }

    // Never attacks
    pub fn passive_towards(self, other: Alignment) -> bool {
        match (self, other) {
            (Alignment::Enemy, Alignment::Enemy) => true,
            (Alignment::Owned(a), Alignment::Owned(b)) if a == b => true,
            (Alignment::Npc, Alignment::Npc) => true,
            (Alignment::Npc, Alignment::Tame) => true,
            (Alignment::Enemy, Alignment::Wild) => true,
            (Alignment::Wild, Alignment::Enemy) => true,
            (Alignment::Tame, Alignment::Npc) => true,
            (Alignment::Tame, Alignment::Tame) => true,
            (_, Alignment::Passive) => true,
            _ => false,
        }
    }
}

impl Component for Alignment {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}

bitflags::bitflags! {
    #[derive(Default)]
    pub struct BehaviorCapability: u8 {
        const SPEAK = 0b00000001;
    }
}
bitflags::bitflags! {
    #[derive(Default)]
    pub struct BehaviorState: u8 {
        const TRADING        = 0b00000001;
        const TRADING_ISSUER = 0b00000010;
    }
}

/// # Behavior Component
/// This component allow an Entity to register one or more behavior tags.
/// These tags act as flags of what an Entity can do, or what it is doing.
/// Behaviors Tags can be added and removed as the Entity lives, to update its
/// state when needed
#[derive(Default, Copy, Clone, Debug)]
pub struct Behavior {
    capabilities: BehaviorCapability,
    state: BehaviorState,
    pub trade_site: Option<SiteId>,
}

impl From<BehaviorCapability> for Behavior {
    fn from(capabilities: BehaviorCapability) -> Self {
        Behavior {
            capabilities,
            state: BehaviorState::default(),
            trade_site: None,
        }
    }
}

impl Behavior {
    /// Builder function
    /// Set capabilities if Option is Some
    #[must_use]
    pub fn maybe_with_capabilities(
        mut self,
        maybe_capabilities: Option<BehaviorCapability>,
    ) -> Self {
        if let Some(capabilities) = maybe_capabilities {
            self.allow(capabilities)
        }
        self
    }

    /// Builder function
    /// Set trade_site if Option is Some
    #[must_use]
    pub fn with_trade_site(mut self, trade_site: Option<SiteId>) -> Self {
        self.trade_site = trade_site;
        self
    }

    /// Set capabilities to the Behavior
    pub fn allow(&mut self, capabilities: BehaviorCapability) {
        self.capabilities.set(capabilities, true)
    }

    /// Unset capabilities to the Behavior
    pub fn deny(&mut self, capabilities: BehaviorCapability) {
        self.capabilities.set(capabilities, false)
    }

    /// Check if the Behavior is able to do something
    pub fn can(&self, capabilities: BehaviorCapability) -> bool {
        self.capabilities.contains(capabilities)
    }

    /// Check if the Behavior is able to trade
    pub fn can_trade(&self) -> bool { self.trade_site.is_some() }

    /// Set a state to the Behavior
    pub fn set(&mut self, state: BehaviorState) { self.state.set(state, true) }

    /// Unset a state to the Behavior
    pub fn unset(&mut self, state: BehaviorState) { self.state.set(state, false) }

    /// Check if the Behavior has a specific state
    pub fn is(&self, state: BehaviorState) -> bool { self.state.contains(state) }
}

#[derive(Clone, Debug, Default)]
pub struct Psyche {
    /// The proportion of health below which entities will start fleeing.
    /// 0.0 = never flees, 1.0 = always flees, 0.5 = flee at 50% health.
    pub flee_health: f32,
    /// The distance below which the agent will see enemies if it has line of
    /// sight.
    pub sight_dist: f32,
    /// The distance below which the agent can hear enemies without seeing them.
    pub listen_dist: f32,
    /// The distance below which the agent will attack enemies. Should be lower
    /// than `sight_dist`. `None` implied that the agent is always aggro
    /// towards enemies that it is aware of.
    pub aggro_dist: Option<f32>,
}

impl<'a> From<&'a Body> for Psyche {
    fn from(body: &'a Body) -> Self {
        Self {
            flee_health: match body {
                Body::Humanoid(humanoid) => match humanoid.species {
                    humanoid::Species::Danari => 0.4,
                    humanoid::Species::Dwarf => 0.3,
                    humanoid::Species::Elf => 0.4,
                    humanoid::Species::Human => 0.4,
                    humanoid::Species::Orc => 0.3,
                    humanoid::Species::Undead => 0.3,
                },
                Body::QuadrupedSmall(quadruped_small) => match quadruped_small.species {
                    quadruped_small::Species::Pig => 0.5,
                    quadruped_small::Species::Fox => 0.7,
                    quadruped_small::Species::Sheep => 0.6,
                    quadruped_small::Species::Boar => 0.1,
                    quadruped_small::Species::Jackalope => 0.0,
                    quadruped_small::Species::Skunk => 0.4,
                    quadruped_small::Species::Cat => 0.9,
                    quadruped_small::Species::Batfox => 0.1,
                    quadruped_small::Species::Raccoon => 0.6,
                    quadruped_small::Species::Dodarock => 0.0,
                    quadruped_small::Species::Holladon => 0.0,
                    quadruped_small::Species::Hyena => 0.2,
                    quadruped_small::Species::Dog => 0.8,
                    quadruped_small::Species::Rabbit => 0.7,
                    quadruped_small::Species::Truffler => 0.2,
                    quadruped_small::Species::Hare => 0.3,
                    quadruped_small::Species::Goat => 0.5,
                    quadruped_small::Species::Porcupine => 0.7,
                    quadruped_small::Species::Turtle => 0.7,
                    // FIXME: This is to balance for enemy rats in dunegeons
                    // Normal rats should probably always flee.
                    quadruped_small::Species::Rat => 0.0,
                    quadruped_small::Species::Beaver => 0.7,
                    _ => 1.0,
                },
                Body::QuadrupedMedium(quadruped_medium) => match quadruped_medium.species {
                    quadruped_medium::Species::Frostfang => 0.1,
                    quadruped_medium::Species::Catoblepas => 0.2,
                    quadruped_medium::Species::Darkhound => 0.1,
                    quadruped_medium::Species::Dreadhorn => 0.2,
                    quadruped_medium::Species::Bonerattler => 0.0,
                    quadruped_medium::Species::Tiger => 0.1,
                    _ => 0.3,
                },
                Body::QuadrupedLow(quadruped_low) => match quadruped_low.species {
                    quadruped_low::Species::Salamander => 0.2,
                    quadruped_low::Species::Monitor => 0.3,
                    quadruped_low::Species::Pangolin => 0.6,
                    quadruped_low::Species::Tortoise => 0.2,
                    quadruped_low::Species::Rocksnapper => 0.05,
                    quadruped_low::Species::Asp => 0.05,
                    _ => 0.0,
                },
                Body::BipedSmall(biped_small) => match biped_small.species {
                    biped_small::Species::Gnarling => 0.2,
                    biped_small::Species::Adlet => 0.2,
                    biped_small::Species::Haniwa => 0.1,
                    biped_small::Species::Sahagin => 0.1,
                    biped_small::Species::Myrmidon => 0.0,
                    biped_small::Species::Husk => 0.0,
                    _ => 0.5,
                },
                Body::BirdMedium(bird_medium) => match bird_medium.species {
                    bird_medium::Species::Goose => 0.4,
                    bird_medium::Species::Peacock => 0.4,
                    bird_medium::Species::Eagle => 0.3,
                    bird_medium::Species::Parrot => 0.8,
                    _ => 0.5,
                },
                Body::BirdLarge(_) => 0.1,
                Body::FishSmall(_) => 1.0,
                Body::FishMedium(_) => 0.75,
                Body::BipedLarge(_) => 0.0,
                Body::Object(_) => 0.0,
                Body::Golem(_) => 0.0,
                Body::Theropod(_) => 0.0,
                Body::Ship(_) => 0.0,
                Body::Dragon(_) => 0.0,
            },
            sight_dist: 40.0,
            listen_dist: 30.0,
            aggro_dist: match body {
                Body::Humanoid(_) => Some(20.0),
                _ => None, // Always aggressive if detected
            },
        }
    }
}

impl Psyche {
    /// The maximum distance that targets might be detected by this agent.
    pub fn search_dist(&self) -> f32 { self.sight_dist.max(self.listen_dist) }
}

#[derive(Clone, Debug)]
/// Events that affect agent behavior from other entities/players/environment
pub enum AgentEvent {
    /// Engage in conversation with entity with Uid
    Talk(Uid, Subject),
    TradeInvite(Uid),
    TradeAccepted(Uid),
    FinishedTrade(TradeResult),
    UpdatePendingTrade(
        // This data structure is large so box it to keep AgentEvent small
        Box<(
            TradeId,
            PendingTrade,
            SitePrices,
            [Option<ReducedInventory>; 2],
        )>,
    ),
    ServerSound(Sound),
    Hurt,
}

#[derive(Copy, Clone, Debug)]
pub struct Sound {
    pub kind: SoundKind,
    pub pos: Vec3<f32>,
    pub vol: f32,
    pub time: f64,
}

impl Sound {
    pub fn new(kind: SoundKind, pos: Vec3<f32>, vol: f32, time: f64) -> Self {
        Sound {
            kind,
            pos,
            vol,
            time,
        }
    }

    #[must_use]
    pub fn with_new_vol(mut self, new_vol: f32) -> Self {
        self.vol = new_vol;

        self
    }
}

#[derive(Copy, Clone, Debug)]
pub enum SoundKind {
    Unknown,
    Movement,
    Melee,
    Projectile,
    Explosion,
    Beam,
    Shockwave,
    Utterance(UtteranceKind, Body),
}

#[derive(Clone, Copy, Debug)]
pub struct Target {
    pub target: EcsEntity,
    /// Whether the target is hostile
    pub hostile: bool,
    /// The time at which the target was selected
    pub selected_at: f64,
    /// Whether the target has come close enough to trigger aggro.
    pub aggro_on: bool,
}

impl Target {
    pub fn new(target: EcsEntity, hostile: bool, selected_at: f64, aggro_on: bool) -> Self {
        Self {
            target,
            hostile,
            selected_at,
            aggro_on,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, EnumIter)]
pub enum TimerAction {
    Interact,
}

/// A time used for managing agent-related timeouts. The timer is designed to
/// keep track of the start of any number of previous actions. However,
/// starting/progressing an action will end previous actions. Therefore, the
/// timer should be used for actions that are mutually-exclusive.
#[derive(Clone, Debug)]
pub struct Timer {
    action_starts: Vec<Option<f64>>,
    last_action: Option<TimerAction>,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            action_starts: TimerAction::iter().map(|_| None).collect(),
            last_action: None,
        }
    }
}

impl Timer {
    fn idx_for(action: TimerAction) -> usize {
        TimerAction::iter()
            .enumerate()
            .find(|(_, a)| a == &action)
            .unwrap()
            .0 // Can't fail, EnumIter is exhaustive
    }

    /// Reset the timer for the given action, returning true if the timer was
    /// not already reset.
    pub fn reset(&mut self, action: TimerAction) -> bool {
        std::mem::replace(&mut self.action_starts[Self::idx_for(action)], None).is_some()
    }

    /// Start the timer for the given action, even if it was already started.
    pub fn start(&mut self, time: f64, action: TimerAction) {
        self.action_starts[Self::idx_for(action)] = Some(time);
        self.last_action = Some(action);
    }

    /// Continue timing the given action, starting it if it was not already
    /// started.
    pub fn progress(&mut self, time: f64, action: TimerAction) {
        if self.last_action != Some(action) {
            self.start(time, action);
        }
    }

    /// Return the time that the given action was last performed at.
    pub fn time_of_last(&self, action: TimerAction) -> Option<f64> {
        self.action_starts[Self::idx_for(action)]
    }

    /// Return `true` if the time since the action was last started exceeds the
    /// given timeout.
    pub fn time_since_exceeds(&self, time: f64, action: TimerAction, timeout: f64) -> bool {
        self.time_of_last(action)
            .map_or(true, |last_time| (time - last_time).max(0.0) > timeout)
    }

    /// Return `true` while the time since the action was last started is less
    /// than the given period. Once the time has elapsed, reset the timer.
    pub fn timeout_elapsed(
        &mut self,
        time: f64,
        action: TimerAction,
        timeout: f64,
    ) -> Option<bool> {
        if self.time_since_exceeds(time, action, timeout) {
            Some(self.reset(action))
        } else {
            self.progress(time, action);
            None
        }
    }
}

#[allow(clippy::type_complexity)]
#[derive(Clone, Debug)]
pub struct Agent {
    pub rtsim_controller: RtSimController,
    pub patrol_origin: Option<Vec3<f32>>,
    pub target: Option<Target>,
    pub chaser: Chaser,
    pub behavior: Behavior,
    pub psyche: Psyche,
    pub inbox: VecDeque<AgentEvent>,
    pub action_state: ActionState,
    pub timer: Timer,
    pub bearing: Vec2<f32>,
    pub sounds_heard: Vec<Sound>,
    pub awareness: f32,
    pub position_pid_controller: Option<PidController<fn(Vec3<f32>, Vec3<f32>) -> f32, 16>>,
}

#[derive(Clone, Debug, Default)]
pub struct ActionState {
    pub timer: f32,
    pub counter: f32,
    pub condition: bool,
    pub int_counter: u8,
}

impl Agent {
    pub fn from_body(body: &Body) -> Self {
        Agent {
            rtsim_controller: RtSimController::default(),
            patrol_origin: None,
            target: None,
            chaser: Chaser::default(),
            behavior: Behavior::default(),
            psyche: Psyche::from(body),
            inbox: VecDeque::new(),
            action_state: ActionState::default(),
            timer: Timer::default(),
            bearing: Vec2::zero(),
            sounds_heard: Vec::new(),
            awareness: 0.0,
            position_pid_controller: None,
        }
    }

    #[must_use]
    pub fn with_patrol_origin(mut self, origin: Vec3<f32>) -> Self {
        self.patrol_origin = Some(origin);
        self
    }

    #[must_use]
    pub fn with_behavior(mut self, behavior: Behavior) -> Self {
        self.behavior = behavior;
        self
    }

    #[must_use]
    pub fn with_no_flee_if(mut self, condition: bool) -> Self {
        if condition {
            self.psyche.flee_health = 0.0;
        }
        self
    }

    // FIXME: Only one of *three* things in this method sets a location.
    #[must_use]
    pub fn with_destination(mut self, pos: Vec3<f32>) -> Self {
        self.psyche.flee_health = 0.0;
        self.rtsim_controller = RtSimController::with_destination(pos);
        self.behavior.allow(BehaviorCapability::SPEAK);
        self
    }

    #[allow(clippy::type_complexity)]
    #[must_use]
    pub fn with_position_pid_controller(
        mut self,
        pid: PidController<fn(Vec3<f32>, Vec3<f32>) -> f32, 16>,
    ) -> Self {
        self.position_pid_controller = Some(pid);
        self
    }

    #[must_use]
    pub fn with_aggro_no_warn(mut self) -> Self {
        self.psyche.aggro_dist = None;
        self
    }

    pub fn decrement_awareness(&mut self, dt: f32) {
        let mut decrement = dt * AWARENESS_DECREMENT_CONSTANT;
        let awareness = self.awareness;

        let too_high = awareness >= 100.0;
        let high = awareness >= 50.0;
        let medium = awareness >= 30.0;
        let low = awareness > 15.0;
        let positive = awareness >= 0.0;
        let negative = awareness < 0.0;

        if too_high {
            decrement *= 3.0;
        } else if high {
            decrement *= 1.0;
        } else if medium {
            decrement *= 2.5;
        } else if low {
            decrement *= 0.70;
        } else if positive {
            decrement *= 0.5;
        } else if negative {
            return;
        }

        self.awareness -= decrement;
    }

    pub fn forget_old_sounds(&mut self, time: f64) {
        if !self.sounds_heard.is_empty() {
            // Keep (retain) only newer sounds
            self.sounds_heard
                .retain(|&sound| time - sound.time <= SECONDS_BEFORE_FORGET_SOUNDS);
        }
    }

    pub fn allowed_to_speak(&self) -> bool { self.behavior.can(BehaviorCapability::SPEAK) }

    pub fn forget_enemy(&mut self, target_name: &str) {
        self.rtsim_controller
            .events
            .push(RtSimEvent::ForgetEnemy(target_name.to_owned()));
    }

    pub fn add_enemy(&mut self, target_name: &str, time: f64) {
        self.rtsim_controller
            .events
            .push(RtSimEvent::AddMemory(Memory {
                item: MemoryItem::CharacterFight {
                    name: target_name.to_owned(),
                },
                time_to_forget: time + 300.0,
            }));
    }
}

impl Component for Agent {
    type Storage = IdvStorage<Self>;
}

#[cfg(test)]
mod tests {
    use super::{Behavior, BehaviorCapability, BehaviorState};

    /// Test to verify that Behavior is working correctly at its most basic
    /// usages
    #[test]
    pub fn behavior_basic() {
        let mut b = Behavior::default();
        // test capabilities
        assert!(!b.can(BehaviorCapability::SPEAK));
        b.allow(BehaviorCapability::SPEAK);
        assert!(b.can(BehaviorCapability::SPEAK));
        b.deny(BehaviorCapability::SPEAK);
        assert!(!b.can(BehaviorCapability::SPEAK));
        // test states
        assert!(!b.is(BehaviorState::TRADING));
        b.set(BehaviorState::TRADING);
        assert!(b.is(BehaviorState::TRADING));
        b.unset(BehaviorState::TRADING);
        assert!(!b.is(BehaviorState::TRADING));
        // test `from`
        let b = Behavior::from(BehaviorCapability::SPEAK);
        assert!(b.can(BehaviorCapability::SPEAK));
    }
}

/// PID controllers are used for automatically adapting nonlinear controls (like
/// buoyancy for airships) to target specific outcomes (i.e. a specific height)
#[derive(Clone)]
pub struct PidController<F: Fn(Vec3<f32>, Vec3<f32>) -> f32, const NUM_SAMPLES: usize> {
    /// The coefficient of the proportional term
    pub kp: f32,
    /// The coefficient of the integral term
    pub ki: f32,
    /// The coefficient of the derivative term
    pub kd: f32,
    /// The setpoint that the process has as its goal
    pub sp: Vec3<f32>,
    /// A ring buffer of the last NUM_SAMPLES measured process variables
    pv_samples: [(f64, Vec3<f32>); NUM_SAMPLES],
    /// The index into the ring buffer of process variables
    pv_idx: usize,
    /// The total integral error
    integral_error: f64,
    /// The error function, to change how the difference between the setpoint
    /// and process variables are calculated
    e: F,
}

impl<F: Fn(Vec3<f32>, Vec3<f32>) -> f32, const NUM_SAMPLES: usize> fmt::Debug
    for PidController<F, NUM_SAMPLES>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("PidController")
            .field("kp", &self.kp)
            .field("ki", &self.ki)
            .field("kd", &self.kd)
            .field("sp", &self.sp)
            .field("pv_samples", &self.pv_samples)
            .field("pv_idx", &self.pv_idx)
            .finish()
    }
}

impl<F: Fn(Vec3<f32>, Vec3<f32>) -> f32, const NUM_SAMPLES: usize> PidController<F, NUM_SAMPLES> {
    /// Constructs a PidController with the specified weights, setpoint,
    /// starting time, and error function
    pub fn new(kp: f32, ki: f32, kd: f32, sp: Vec3<f32>, time: f64, e: F) -> Self {
        Self {
            kp,
            ki,
            kd,
            sp,
            pv_samples: [(time, sp); NUM_SAMPLES],
            pv_idx: 0,
            integral_error: 0.0,
            e,
        }
    }

    /// Adds a measurement of the process variable to the ringbuffer
    pub fn add_measurement(&mut self, time: f64, pv: Vec3<f32>) {
        self.pv_idx += 1;
        self.pv_idx %= NUM_SAMPLES;
        self.pv_samples[self.pv_idx] = (time, pv);
        self.update_integral_err();
    }

    /// The amount to set the control variable to is a weighed sum of the
    /// proportional error, the integral error, and the derivative error.
    /// https://en.wikipedia.org/wiki/PID_controller#Mathematical_form
    pub fn calc_err(&self) -> f32 {
        self.kp * self.proportional_err()
            + self.ki * self.integral_err()
            + self.kd * self.derivative_err()
    }

    /// The proportional error is the error function applied to the set point
    /// and the most recent process variable measurement
    pub fn proportional_err(&self) -> f32 { (self.e)(self.sp, self.pv_samples[self.pv_idx].1) }

    /// The integral error is the error function integrated over all previous
    /// values, updated per point. The trapezoid rule for numerical integration
    /// was chosen because it's fairly easy to calculate and sufficiently
    /// accurate. https://en.wikipedia.org/wiki/Trapezoidal_rule#Uniform_grid
    pub fn integral_err(&self) -> f32 { self.integral_error as f32 }

    fn update_integral_err(&mut self) {
        let f = |x| (self.e)(self.sp, x) as f64;
        let (a, x0) = self.pv_samples[(self.pv_idx + NUM_SAMPLES - 1) % NUM_SAMPLES];
        let (b, x1) = self.pv_samples[self.pv_idx];
        let dx = b - a;
        // Discard updates with too long between them, likely caused by either
        // initialization or latency, since they're likely to be spurious
        if dx < 5.0 {
            self.integral_error += dx * (f(x1) + f(x0)) / 2.0;
        }
    }

    /// The derivative error is the numerical derivative of the error function
    /// based on the most recent 2 samples. Using more than 2 samples might
    /// improve the accuracy of the estimate of the derivative, but it would be
    /// an estimate of the derivative error further in the past.
    /// https://en.wikipedia.org/wiki/Numerical_differentiation#Finite_differences
    pub fn derivative_err(&self) -> f32 {
        let f = |x| (self.e)(self.sp, x);
        let (a, x0) = self.pv_samples[(self.pv_idx + NUM_SAMPLES - 1) % NUM_SAMPLES];
        let (b, x1) = self.pv_samples[self.pv_idx];
        let h = b - a;
        (f(x1) - f(x0)) / h as f32
    }
}

/// Get the PID coefficients associated with some Body, since it will likely
/// need to be tuned differently for each body type
pub fn pid_coefficients(body: &Body) -> (f32, f32, f32) {
    match body {
        Body::Ship(ship::Body::DefaultAirship) => {
            let kp = 1.0;
            let ki = 0.1;
            let kd = 1.2;
            (kp, ki, kd)
        },
        Body::Ship(ship::Body::AirBalloon) => {
            let kp = 1.0;
            let ki = 0.1;
            let kd = 0.8;
            (kp, ki, kd)
        },
        // default to a pure-proportional controller, which is the first step when tuning
        _ => (1.0, 0.0, 0.0),
    }
}
