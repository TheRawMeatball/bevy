use bevy_ecs::{Entity, Query};
use bevy_math::Vec2;
use bevy_text::CalculatedSize;
use bevy_transform::components::Children;

use crate::{
    Alignment, AnchorLayout, Aspect, AxisConstraint, Constraint, ConstraintSize, Direction,
    MinSize, SpreadConstraint,
};

pub fn solve(
    node: Entity,
    nodes: &Query<(&AnchorLayout, Option<&Children>, Option<&CalculatedSize>)>,
    mutable: &mut Query<&mut MinSize>,
) -> Vec2 {
    let (layout, children, calculated_size) = nodes.get(node).unwrap();

    let inherent_size = calculated_size
        .map(|cs| cs.size.into())
        .unwrap_or_else(Vec2::zero);

    let mut internal_size: Vec2 = if let Some(children) = children {
        match &layout.children_spread {
            SpreadConstraint::None => children.iter().fold(inherent_size, |mut state, &c| {
                let c = solve(c, nodes, mutable);
                state.x = state.x.max(c.x);
                state.y = state.y.max(c.y);
                state
            }),
            SpreadConstraint::Directed { margin, direction } => {
                let (mut internal, count) =
                    children
                        .iter()
                        .fold((Vec2::zero(), 0), |(mut state, count), &c| {
                            let (node, _, _) = nodes.get(c).unwrap();
                            let internal_size = solve(c, nodes, mutable);
                            let cc = node.child_constraint.as_ref().unwrap();
                            match direction {
                                Direction::Up | Direction::Down => {
                                    let aligned = match cc.min_size {
                                        ConstraintSize::Pixels(v) => v,
                                        ConstraintSize::FromContent => internal_size.y,
                                    };
                                    let perp = internal_size.x;
                                    state.x = state.x.max(perp);
                                    state.y += aligned;
                                }
                                Direction::Left | Direction::Right => {
                                    let aligned = match cc.min_size {
                                        ConstraintSize::Pixels(v) => v,
                                        ConstraintSize::FromContent => internal_size.x,
                                    };
                                    let perp = internal_size.y;
                                    state.x += aligned;
                                    state.y = state.y.max(perp);
                                }
                            };
                            (state, count + 1)
                        });

                let margins = (count - 1).max(0) as f32 * margin;
                match direction {
                    Direction::Left | Direction::Right => internal.x += margins,
                    Direction::Up | Direction::Down => internal.y += margins,
                }
                internal.max(inherent_size)
            }
        }
    } else {
        inherent_size
    };

    internal_size.x += layout.padding.left + layout.padding.right;
    internal_size.y += layout.padding.top + layout.padding.bottom;

    let mut min_size = mutable.get_mut(node).unwrap();
    // Directly changing value avoided to avoid tripping the mutated flag
    if min_size.size != internal_size {
        min_size.size = internal_size;
    }

    match &layout.constraint {
        Constraint::Independent { x, y } => {
            let x = x.solve_min(internal_size.x);
            let y = y.solve_min(internal_size.y);
            Vec2::new(x, y)
        }
        Constraint::SetXWithY { x, y, aspect } => {
            let y = y.solve_min(internal_size.y);
            let x = if let Aspect::Value(v) = aspect {
                x.solve_min(y * v)
            } else {
                let aspect = internal_size.x / internal_size.y;
                if !aspect.is_finite() {
                    0.
                } else {
                    x.solve_min(y * aspect)
                }
            };
            Vec2::new(x, y)
        }
        Constraint::SetYWithX { x, y, aspect } => {
            let x = x.solve_min(internal_size.x);
            let y = if let Aspect::Value(v) = aspect {
                y.solve_min(x / v)
            } else {
                let aspect = internal_size.x / internal_size.y;
                if !aspect.is_finite() {
                    0.
                } else {
                    y.solve_min(x / aspect)
                }
            };
            Vec2::new(x, y)
        }
        Constraint::MaxAspect(aspect) => {
            if let Aspect::Value(v) = aspect {
                let x_comparable = internal_size.x / v;
                if x_comparable > internal_size.y {
                    Vec2::new(internal_size.x, x_comparable)
                } else {
                    Vec2::new(v * internal_size.y, internal_size.y)
                }
            } else {
                internal_size
            }
        }
    }
}

impl AxisConstraint {
    fn solve_min(&self, internal_size: f32) -> f32 {
        match self {
            AxisConstraint::DirectMarginAndSize(m, size)
            | AxisConstraint::ReverseMarginAndSize(m, size) => m + size,
            AxisConstraint::Centered(size) => *size,

            AxisConstraint::DoubleMargin(m1, m2) => m1 + m2 + internal_size,
            AxisConstraint::FromContentSize(a) => a.solve_min(internal_size),
        }
    }
}

impl Alignment {
    fn solve_min(&self, children_min: f32) -> f32 {
        match self {
            Alignment::DirectMargin(m) | Alignment::ReverseMargin(m) => children_min + m,
            Alignment::Offset(_) => children_min,
        }
    }
}
