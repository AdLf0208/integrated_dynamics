// Pure logic for Integrated Dynamics slice 1: the Logic Cable's six part
// slots and the redstone channel that a Redstone Reader feeds and a Redstone
// Writer emits.
//
// This module has no world access: the support module reads the cable's state
// and block entity, the reader's neighbouring signal and the connected cable
// set, and calls these rules. The face order is Horizon's `Direction` order
// (`down, up, north, south, west, east`), so `face as u32` maps straight onto
// the engine's direction.
//
// Upstream (`PartTypeRedstoneReader`, `PartTypeRedstoneWriter`,
// `AspectReadBuilders.Redstone`, `AspectWriteBuilders.Redstone`) has a reader
// read the redstone level its target block gives toward the part, a writer set
// that level on its target, and status active/inactive from the bound aspect.
// This slice carries the integer channel without the variable-card layer.

/// The six faces of a Logic Cable, in `Direction` order.
pub const FACE_COUNT: usize = 6;

/// Face names, in `Direction` order (the blockstate property names).
pub const FACE_NAMES: [&str; FACE_COUNT] = ["down", "up", "north", "south", "west", "east"];

/// A part installed on one face of a cable. Upstream's cable block entity holds
/// one part per face (`BlockEntityMultipartTicking`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Part {
    /// No part on this face.
    #[default]
    None,
    /// A Redstone Reader: samples the level reaching the cable into the channel.
    Reader,
    /// A Redstone Writer: emits the channel's level on this face.
    Writer,
    /// A World Reader: samples a world property (light, weather, the day cycle)
    /// of the block on its face into the channel.
    WorldReader,
}

impl Part {
    /// The part a blockstate property value names; `None` for an unknown value.
    pub fn from_name(name: &str) -> Option<Part> {
        match name {
            "none" => Some(Part::None),
            "reader" => Some(Part::Reader),
            "writer" => Some(Part::Writer),
            "world_reader" => Some(Part::WorldReader),
            _ => None,
        }
    }

    /// The blockstate property value for this part.
    pub fn name(self) -> &'static str {
        match self {
            Part::None => "none",
            Part::Reader => "reader",
            Part::Writer => "writer",
            Part::WorldReader => "world_reader",
        }
    }
}

/// The reader's / writer's status, as `PartStateWriterBase` reports it: active
/// while the part has a value to work with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Inactive,
    Active,
}

impl Status {
    /// The blockstate property value.
    pub fn name(self) -> &'static str {
        match self {
            Status::Inactive => "inactive",
            Status::Active => "active",
        }
    }
}

/// The six part slots of one cable, in [`FACE_NAMES`] order.
///
/// Named `PartSlots`, not `Parts`: the SDK has its own (`Parts`, the attached
/// parts by target id) and `horizon-mod fits` resolves a source type name against
/// the SDK's surface, so the older name reported a false "no longer builds with a
/// literal" against every other Nightly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PartSlots([Part; FACE_COUNT]);

impl PartSlots {
    /// No parts on any face.
    pub fn new() -> PartSlots {
        PartSlots::default()
    }

    /// The part on `face`, or `None` for an out-of-range face.
    pub fn get(self, face: usize) -> Option<Part> {
        self.0.get(face).copied()
    }

    /// The same cable with `part` on `face` (out-of-range faces are ignored).
    pub fn with(self, face: usize, part: Part) -> PartSlots {
        let mut slots = self.0;
        if let Some(slot) = slots.get_mut(face) {
            *slot = part;
        }
        PartSlots(slots)
    }

    /// Whether any face carries a writer (the cable is then a signal source).
    pub fn has_writer(self) -> bool {
        self.0.contains(&Part::Writer)
    }

    /// Whether any face carries a part at all (as upstream's multipart container
    /// answers `hasParts`), whatever the part reads or writes.
    pub fn has_part(self) -> bool {
        self.0.iter().any(|part| *part != Part::None)
    }

    /// The faces carrying a reader, in face order.
    pub fn reader_faces(&self) -> impl Iterator<Item = usize> {
        self.0
            .iter()
            .enumerate()
            .filter(|(_, part)| **part == Part::Reader)
            .map(|(face, _)| face)
    }

    /// The faces carrying a world reader, in face order.
    pub fn world_reader_faces(&self) -> impl Iterator<Item = usize> {
        self.0
            .iter()
            .enumerate()
            .filter(|(_, part)| **part == Part::WorldReader)
            .map(|(face, _)| face)
    }
}

/// A World Reader aspect: the part of upstream's `Aspects.Read.World` whose value
/// fits the network's redstone channel (a boolean as 0/15, the light level as
/// 0..15). The eight aspects that carry an arbitrary integer, a long, a double, a
/// string or a list (`raincountdown`, `ticktime`, `daytime`, `tps`, `time`,
/// `totaltime`, `name`, `players`) need the variable system's typed values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldAspect {
    /// `read.integer.world.lightlevel`: the target block's brightest local light.
    Light,
    /// `read.boolean.world.weather.clear`: the level is not raining.
    WeatherClear,
    /// `read.boolean.world.weather.raining`: the level is raining.
    WeatherRaining,
    /// `read.boolean.world.weather.thunder`: the level is thundering.
    WeatherThunder,
    /// `read.boolean.world.isday`: vanilla's `Level.isDay`.
    IsDay,
    /// `read.boolean.world.isnight`: the inverse of `Level.isDay`.
    IsNight,
}

impl WorldAspect {
    /// Every aspect this slice carries, in the order `info` lists them.
    pub const ALL: [WorldAspect; 6] = [
        WorldAspect::Light,
        WorldAspect::WeatherClear,
        WorldAspect::WeatherRaining,
        WorldAspect::WeatherThunder,
        WorldAspect::IsDay,
        WorldAspect::IsNight,
    ];

    /// The aspect a saved aspect name selects; `None` for a name this slice does
    /// not carry (including `none`, a face that never had one chosen).
    pub fn from_name(name: &str) -> Option<WorldAspect> {
        match name {
            "light" => Some(WorldAspect::Light),
            "weather_clear" => Some(WorldAspect::WeatherClear),
            "weather_raining" => Some(WorldAspect::WeatherRaining),
            "weather_thunder" => Some(WorldAspect::WeatherThunder),
            "isday" => Some(WorldAspect::IsDay),
            "isnight" => Some(WorldAspect::IsNight),
            _ => None,
        }
    }

    /// The saved name of this aspect (the cable's `aspect_<face>` value).
    pub fn name(self) -> &'static str {
        match self {
            WorldAspect::Light => "light",
            WorldAspect::WeatherClear => "weather_clear",
            WorldAspect::WeatherRaining => "weather_raining",
            WorldAspect::WeatherThunder => "weather_thunder",
            WorldAspect::IsDay => "isday",
            WorldAspect::IsNight => "isnight",
        }
    }
}

/// What a World Reader samples from the level once per level tick: the world
/// facts its aspects read. The support module fills it from the level; the rules
/// here stay free of game access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldView {
    /// The brightest local light at the block the reader faces, 0 to 15.
    pub light: u8,
    /// Whether the level is raining (`Level.isRaining`).
    pub raining: bool,
    /// Whether the level is thundering (`Level.isThundering`).
    pub thundering: bool,
    /// Whether it is day (`Level.isDay`: no fixed time and sky darkening below 4).
    pub day: bool,
}

/// A redstone level for a boolean reading: 15 for true, 0 for false, which is
/// upstream's boolean-to-redstone conversion when a boolean feeds a writer.
const BOOLEAN_LEVEL: u8 = 15;

/// The redstone level a World Reader aspect reads from `view`.
pub fn world_reading(aspect: WorldAspect, view: WorldView) -> u8 {
    fn level(on: bool) -> u8 {
        if on { BOOLEAN_LEVEL } else { 0 }
    }
    match aspect {
        WorldAspect::Light => view.light.min(15),
        WorldAspect::WeatherClear => level(!view.raining),
        WorldAspect::WeatherRaining => level(view.raining),
        WorldAspect::WeatherThunder => level(view.thundering),
        WorldAspect::IsDay => level(view.day),
        WorldAspect::IsNight => level(!view.day),
    }
}

/// The direction offset of `face` in [`FACE_NAMES`] order.
pub fn face_normal(face: usize) -> [i32; 3] {
    match face {
        0 => [0, -1, 0],
        1 => [0, 1, 0],
        2 => [0, 0, -1],
        3 => [0, 0, 1],
        4 => [-1, 0, 0],
        _ => [1, 0, 0],
    }
}

/// The face whose normal is `normal` (the engine's `Direction::as_u32` order),
/// or `None` for a zero/unknown normal.
pub fn face_of_normal(normal: [i32; 3]) -> Option<usize> {
    (0..FACE_COUNT).find(|&face| face_normal(face) == normal)
}

/// A block position in the level.
pub type Pos = [i32; 3];

/// The six positions touching `pos`, in [`FACE_NAMES`] order.
pub fn neighbours(pos: Pos) -> [Pos; FACE_COUNT] {
    std::array::from_fn(|face| {
        let [dx, dy, dz] = face_normal(face);
        [pos[0] + dx, pos[1] + dy, pos[2] + dz]
    })
}

/// The connected cable set starting at `start`: `is_cable` decides whether a
/// position joins it, and at most `limit` cables are collected (a safety cap
/// against a runaway feedback loop, as upstream's network has no size limit).
///
/// Only confirmed cables are remembered and counted: the bound is a *cable*
/// count. Counting every position the walk probed instead let five non-cable
/// neighbours per cable fill the cap, so a long straight cable was cut off at a
/// fifth of the documented 1024 cables (the reviewer's finding).
pub fn collect_network(
    start: Pos,
    mut is_cable: impl FnMut(Pos) -> bool,
    limit: usize,
) -> Vec<Pos> {
    if limit == 0 || !is_cable(start) {
        return Vec::new();
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut queue = std::collections::VecDeque::new();
    seen.insert(start);
    queue.push_back(start);
    let mut out = Vec::new();
    while let Some(pos) = queue.pop_front() {
        if out.len() >= limit {
            break;
        }
        out.push(pos);
        for next in neighbours(pos) {
            if seen.len() >= limit {
                break;
            }
            if seen.contains(&next) || !is_cable(next) {
                continue;
            }
            seen.insert(next);
            queue.push_back(next);
        }
    }
    out
}

/// The channel value a set of reader readings yields: the strongest reading
/// clamped to a redstone level. Upstream binds one variable per reader; this
/// slice has one integer channel per cable network.
pub fn channel_value(readings: &[i32]) -> u8 {
    readings.iter().copied().max().unwrap_or(0).clamp(0, 15) as u8
}

/// The weak redstone signal the cable gives on `face`: the channel value on a
/// writer face, zero everywhere else. Upstream's writer aspect is weak by
/// default (`direct = false`); a strong answer is not carried in this slice.
pub fn signal_for_face(parts: PartSlots, value: u8, face: usize, direct: bool) -> i32 {
    if direct {
        return 0;
    }
    match parts.get(face) {
        Some(Part::Writer) => i32::from(value.min(15)),
        _ => 0,
    }
}

/// Whether the cable is a redstone signal source at all (it has a writer).
pub fn is_signal_source(parts: PartSlots) -> bool {
    parts.has_writer()
}

/// The cable's part status: a cable with a part (reader or writer) is active
/// while the network carries a non-zero value, as upstream's bound part is
/// active while its aspect reads or writes.
pub fn status_for(parts: PartSlots, value: u8) -> Status {
    if parts.has_part() && value > 0 {
        Status::Active
    } else {
        Status::Inactive
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn part_names_round_trip() {
        for part in [Part::None, Part::Reader, Part::Writer] {
            assert_eq!(Part::from_name(part.name()), Some(part));
        }
        assert_eq!(Part::from_name("display"), None);
    }

    #[test]
    fn a_world_reader_is_a_known_part() {
        // Upstream's `PartTypeWorldReader` is placed like any other part item; the cable's
        // state table must name it, or `/setblock` and the item click cannot place one.
        assert!(
            Part::from_name("world_reader").is_some(),
            "the cable's state must accept a world reader"
        );
    }

    #[test]
    fn a_world_reader_keeps_its_cable_active() {
        // `status_for` counts a writer or a redstone reader as "has a part". A world reader is
        // a part too: upstream's `PartStateWriterBase` shows active while its aspect reads.
        assert_eq!(
            status_for(PartSlots::new().with(2, Part::WorldReader), 12),
            Status::Active,
            "a cable carrying only a world reader still has a part"
        );
    }

    #[test]
    fn the_six_world_aspects_round_trip_their_names() {
        let names: Vec<&str> = WorldAspect::ALL
            .iter()
            .map(|aspect| aspect.name())
            .collect();
        assert_eq!(
            names,
            vec![
                "light",
                "weather_clear",
                "weather_raining",
                "weather_thunder",
                "isday",
                "isnight"
            ]
        );
        for aspect in WorldAspect::ALL {
            assert_eq!(WorldAspect::from_name(aspect.name()), Some(aspect));
        }
        // The aspects that need the typed value layer do not parse here.
        assert_eq!(WorldAspect::from_name("players"), None);
        assert_eq!(WorldAspect::from_name("none"), None);
    }

    #[test]
    fn a_world_reading_is_the_aspects_redstone_level() {
        let rainy_night = WorldView {
            light: 9,
            raining: true,
            thundering: false,
            day: false,
        };
        assert_eq!(world_reading(WorldAspect::Light, rainy_night), 9);
        assert_eq!(world_reading(WorldAspect::WeatherRaining, rainy_night), 15);
        assert_eq!(world_reading(WorldAspect::WeatherClear, rainy_night), 0);
        assert_eq!(world_reading(WorldAspect::WeatherThunder, rainy_night), 0);
        assert_eq!(world_reading(WorldAspect::IsDay, rainy_night), 0);
        assert_eq!(world_reading(WorldAspect::IsNight, rainy_night), 15);

        let clear_day = WorldView {
            light: 15,
            raining: false,
            thundering: false,
            day: true,
        };
        assert_eq!(world_reading(WorldAspect::WeatherClear, clear_day), 15);
        assert_eq!(world_reading(WorldAspect::WeatherRaining, clear_day), 0);
        assert_eq!(world_reading(WorldAspect::IsDay, clear_day), 15);
        assert_eq!(world_reading(WorldAspect::IsNight, clear_day), 0);

        // A storm is its own reading; vanilla sets rain with it, and the aspects
        // report each property independently as upstream's `isRaining`/`isThundering` do.
        let storm = WorldView {
            light: 0,
            raining: true,
            thundering: true,
            day: true,
        };
        assert_eq!(world_reading(WorldAspect::WeatherThunder, storm), 15);
        // A stored light above 15 (impossible from the level query) still clamps.
        assert_eq!(
            world_reading(
                WorldAspect::Light,
                WorldView {
                    light: 20,
                    ..clear_day
                }
            ),
            15
        );
    }

    #[test]
    fn faces_are_the_engine_direction_order() {
        assert_eq!(FACE_NAMES, ["down", "up", "north", "south", "west", "east"]);
        assert_eq!(face_normal(0), [0, -1, 0]);
        assert_eq!(face_normal(1), [0, 1, 0]);
        assert_eq!(face_normal(2), [0, 0, -1]);
        assert_eq!(face_normal(3), [0, 0, 1]);
        assert_eq!(face_normal(4), [-1, 0, 0]);
        assert_eq!(face_normal(5), [1, 0, 0]);
        for face in 0..FACE_COUNT {
            assert_eq!(face_of_normal(face_normal(face)), Some(face));
        }
        assert_eq!(face_of_normal([0, 0, 0]), None);
    }

    #[test]
    fn a_part_is_placed_on_one_face_only() {
        let parts = PartSlots::new().with(5, Part::Reader).with(3, Part::Writer);
        assert_eq!(parts.get(5), Some(Part::Reader));
        assert_eq!(parts.get(3), Some(Part::Writer));
        assert_eq!(parts.get(0), Some(Part::None));
        // Replacing a face keeps the others.
        let replaced = parts.with(5, Part::None);
        assert_eq!(replaced.get(5), Some(Part::None));
        assert_eq!(replaced.get(3), Some(Part::Writer));
        // Out-of-range faces do nothing.
        assert_eq!(parts.with(9, Part::Writer), parts);
    }

    #[test]
    fn writer_faces_are_listed_in_face_order() {
        let parts = PartSlots::new()
            .with(5, Part::Writer)
            .with(1, Part::Reader)
            .with(2, Part::Writer);
        assert_eq!(parts.reader_faces().collect::<Vec<_>>(), vec![1]);
        assert!(parts.has_writer());
        assert!(!PartSlots::new().with(2, Part::Reader).has_writer());
        // The world readers are listed on their own: the redstone read path must not
        // sample a world reader's face, and the world path must find only its own.
        let mixed = PartSlots::new()
            .with(0, Part::WorldReader)
            .with(3, Part::Reader)
            .with(4, Part::WorldReader);
        assert_eq!(mixed.world_reader_faces().collect::<Vec<_>>(), vec![0, 4]);
        assert_eq!(mixed.reader_faces().collect::<Vec<_>>(), vec![3]);
        assert!(mixed.has_part());
        assert!(PartSlots::new().with(0, Part::WorldReader).has_part());
    }

    #[test]
    fn a_writer_emits_the_channel_on_its_own_face_only() {
        let parts = PartSlots::new().with(3, Part::Writer).with(4, Part::Reader);
        assert_eq!(signal_for_face(parts, 7, 3, false), 7);
        // A reader face gives nothing.
        assert_eq!(signal_for_face(parts, 7, 4, false), 0);
        // An empty face gives nothing.
        assert_eq!(signal_for_face(parts, 7, 0, false), 0);
        // A strong (direct) answer is not carried in this slice.
        assert_eq!(signal_for_face(parts, 7, 3, true), 0);
        // Values are clamped to a redstone level.
        assert_eq!(signal_for_face(parts, 99, 3, false), 15);
        assert_eq!(signal_for_face(parts, 0, 3, false), 0);
    }

    #[test]
    fn only_a_cable_with_a_writer_is_a_signal_source() {
        assert!(is_signal_source(PartSlots::new().with(0, Part::Writer)));
        assert!(!is_signal_source(PartSlots::new().with(0, Part::Reader)));
        assert!(!is_signal_source(PartSlots::new()));
    }

    #[test]
    fn the_channel_takes_the_strongest_reader_level() {
        assert_eq!(channel_value(&[]), 0);
        assert_eq!(channel_value(&[0, 0, 0]), 0);
        assert_eq!(channel_value(&[3, 11, 7]), 11);
        assert_eq!(channel_value(&[15, 2]), 15);
        // Out-of-range readings are clamped to a redstone level.
        assert_eq!(channel_value(&[-4, 99]), 15);
    }

    #[test]
    fn the_status_follows_the_writer_and_its_value() {
        let writer = PartSlots::new().with(2, Part::Writer);
        assert_eq!(status_for(writer, 0), Status::Inactive);
        assert_eq!(status_for(writer, 1), Status::Active);
        // A reader-only cable is active while it reads a value.
        let reader = PartSlots::new().with(2, Part::Reader);
        assert_eq!(status_for(reader, 12), Status::Active);
        assert_eq!(status_for(reader, 0), Status::Inactive);
        // A cable with no part is never active.
        assert_eq!(status_for(PartSlots::new(), 12), Status::Inactive);
    }

    #[test]
    fn the_network_is_the_connected_cable_set() {
        // A line of cables x = 0..=2, with a hole at x = 3.
        let cables = |pos: Pos| pos[1] == 0 && pos[2] == 0 && (0..=2).contains(&pos[0]);
        let mut network = collect_network([0, 0, 0], cables, 64);
        network.sort();
        assert_eq!(network, vec![[0, 0, 0], [1, 0, 0], [2, 0, 0]]);
        // A cable that is not connected in is not reached.
        assert!(
            !collect_network([0, 0, 0], cables, 64).contains(&[5, 0, 0]),
            "a disconnected cable must not join the network"
        );
    }

    #[test]
    fn the_network_is_bounded_against_a_runaway_loop() {
        // An infinite plane of cables: the safety cap stops the walk.
        let plane = |_pos: Pos| true;
        assert_eq!(collect_network([0, 0, 0], plane, 25).len(), 25);
    }

    #[test]
    fn a_sparse_line_keeps_its_whole_cable_budget() {
        // A long cable running east, in empty space: every cable has five
        // neighbours that are not cables. The cap counts cables, so all of them
        // join up to the limit — counting probed positions instead cut the line
        // off at about a fifth of it.
        let line = |pos: Pos| pos[1] == 0 && pos[2] == 0 && (0..600).contains(&pos[0]);
        let network = collect_network([0, 0, 0], line, 512);
        assert_eq!(
            network.len(),
            512,
            "512 cables in a row are all in the network"
        );
        assert!(network.contains(&[511, 0, 0]), "and the far end is reached");
    }

    #[test]
    fn a_block_that_is_not_a_cable_starts_no_network() {
        assert!(collect_network([0, 0, 0], |_pos| false, 64).is_empty());
        assert!(collect_network([0, 0, 0], |_pos| true, 0).is_empty());
    }
}
