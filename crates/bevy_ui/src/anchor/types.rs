use crate::Anchors;
use bevy_math::Vec2;

#[derive(Clone, Debug, Default)]
pub struct AnchorLayout {
    pub anchors: Anchors,
    pub constraint: Constraint,
    pub children_spread: Option<SpreadConstraint>,
    pub child_constraint: Option<ChildConstraint>,
}

#[derive(Clone, Debug, Default)]
pub struct ANodeLayoutCache {
    pub(crate) sizes: Option<Vec<Vec2>>,
}

#[derive(Clone, Debug)]
pub enum Constraint {
    Independent {
        x: AxisConstraint,
        y: AxisConstraint,
    },
    SetXWithY {
        x: Alignment,
        y: AxisConstraint,
        aspect: Option<f32>,
    },
    SetYWithX {
        x: AxisConstraint,
        y: Alignment,
        aspect: Option<f32>,
    },
    MaxAspect(f32),
}

impl Default for Constraint {
    fn default() -> Self {
        Constraint::Independent {
            x: Default::default(),
            y: Default::default(),
        }
    }
}

// Maybe make this an enum and implement subset of flexbox / css grid?
#[derive(Clone, Debug)]
pub struct ChildConstraint {
    pub weight: f32,
    pub min_size: f32,
    pub max_size: f32,
}

impl Default for ChildConstraint {
    fn default() -> Self {
        Self {
            weight: 1.,
            min_size: 0.,
            max_size: f32::MAX,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SpreadConstraint {
    pub margin: f32,
    pub direction: Direction,
    pub __cache: Vec<Vec2>,
}

#[derive(Clone, Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Default for Direction {
    fn default() -> Self {
        Direction::Right
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AxisConstraint {
    DoubleMargin(f32, f32),
    DirectMarginAndSize(f32, f32),
    ReverseMarginAndSize(f32, f32),
    Centered(f32),
    /// This is only valid for elements with `CalculatedSize`
    FromContentSize(Alignment),
}

#[derive(Debug, Clone, Copy)]
pub enum Alignment {
    DirectMargin(f32),
    ReverseMargin(f32),
    Offset(f32),
}

impl Default for AxisConstraint {
    fn default() -> Self {
        AxisConstraint::DoubleMargin(0., 0.)
    }
}
