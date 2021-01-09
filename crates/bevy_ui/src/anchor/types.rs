use crate::Anchors;
use bevy_math::{Rect, Vec2};

#[derive(Clone, Debug, Default)]
pub struct AnchorLayout {
    pub anchors: Anchors,
    pub constraint: Constraint,
    pub padding: Rect<f32>,
    pub children_spread: Option<SpreadConstraint>,
    pub child_constraint: Option<ChildConstraint>,
}

#[derive(Clone, Debug, Default)]
pub struct LayoutCache {
    /// Used by SpreadConstraint to cache children sizes
    pub(crate) children_sizes: Option<Vec<Vec2>>,
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
        aspect: Aspect,
    },
    SetYWithX {
        x: AxisConstraint,
        y: Alignment,
        aspect: Aspect,
    },
    MaxAspect(Aspect),
}

#[derive(Copy, Clone, Debug)]
pub enum Aspect {
    Value(f32),
    /// This is only valid for elements with `CalculatedSize`
    FromContentSize,
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
    pub min_size: ConstraintSize,
    pub max_size: ConstraintSize,
}

#[derive(Copy, Clone, Debug)]
pub enum ConstraintSize {
    Pixels(f32),
    FromContent,
}

impl ConstraintSize {
    pub(crate) fn unwrap_or_else(self, f: impl FnOnce() -> f32) -> f32 {
        match self {
            ConstraintSize::Pixels(p) => p,
            ConstraintSize::FromContent => f(),
        }
    }
}

impl Default for ChildConstraint {
    fn default() -> Self {
        Self {
            weight: 1.,
            min_size: ConstraintSize::Pixels(0.),
            max_size: ConstraintSize::Pixels(f32::MAX),
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
