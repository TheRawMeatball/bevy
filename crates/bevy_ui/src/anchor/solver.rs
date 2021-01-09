use std::collections::BTreeMap;

use bevy_core::FloatOrd;
use bevy_ecs::{Entity, Flags, Query};
use bevy_math::{Rect, Vec2, Vec3};
use bevy_text::CalculatedSize;
use bevy_transform::components::{Children, Transform};

use crate::{MinSize, Node};

use super::*;

pub(crate) fn solve(
    solve_entity: Entity,
    parent_size: Vec2,
    parent_padding: Rect<f32>,
    respect_flags: bool,
    nodes: &Query<(
        &AnchorLayout,
        Flags<AnchorLayout>,
        &MinSize,
        Flags<MinSize>,
        Option<&CalculatedSize>,
        Option<&Children>,
        Option<Flags<Children>>,
    )>,
    mutables: &mut Query<(&mut Transform, &mut Node, &mut LayoutCache), With<AnchorLayout>>,
) {
    let (mut target_transform, mut node, cache) = mutables.get_mut(solve_entity).unwrap();
    let target_size = &mut node.size;
    let (solve_layout, layout_flags, min_size, min_size_flags, c_size, children, children_flags) =
        nodes.get(solve_entity).unwrap();

    let min_size = min_size.size;

    // <caching>
    if respect_flags
        && !layout_flags.changed()
        && !min_size_flags.changed()
        && !c_size.map(|f| f.dirty).unwrap_or(false)
    {
        if let Some(children) = children {
            let solve_self = |transforms| {
                solve(
                    solve_entity,
                    parent_size,
                    parent_padding,
                    false,
                    nodes,
                    transforms,
                )
            };
            let ts = *target_size;
            if solve_layout.children_spread.is_some() {
                if children_flags.unwrap().changed() {
                    solve_self(mutables);
                    return;
                }
                for child in children.iter() {
                    let (_, layout_flags, _, min_size, c_size, ..) = nodes.get(*child).unwrap();
                    if layout_flags.changed()
                        || min_size.changed()
                        || c_size.map(|cs| cs.dirty).unwrap_or(false)
                    {
                        solve_self(mutables);
                        return;
                    }
                }
                let cache = cache.children_sizes.as_ref().unwrap().clone();
                for (child, size) in children.iter().zip(cache.iter()) {
                    solve(*child, *size, solve_layout.padding, true, nodes, mutables)
                }
            } else {
                for child in children.iter() {
                    solve(*child, ts, solve_layout.padding, true, nodes, mutables)
                }
            }
        }
        return;
    }
    // </caching>

    let parent_size = parent_size
        - Vec2::new(
            parent_padding.left + parent_padding.right,
            parent_padding.top + parent_padding.bottom,
        );

    let mut offset = match &solve_layout.constraint {
        Constraint::Independent { x, y } => {
            let x = x.solve(solve_layout.anchors.x(), parent_size.x, min_size.x);
            let y = y.solve(solve_layout.anchors.y(), parent_size.y, min_size.y);

            *target_size = Vec2::new(x.size, y.size);
            Vec2::new(x.offset, y.offset)
        }
        Constraint::SetXWithY { x, y, aspect } => {
            let y = y.solve(solve_layout.anchors.y(), parent_size.y, min_size.y);
            let aspect = aspect.unwrap_or_else(|| {
                c_size
                    .map(|cs| cs.size.width / cs.size.height)
                    .unwrap_or(1.)
            });
            let x = x.solve(aspect, y.size, parent_size.x, solve_layout.anchors.x());

            *target_size = Vec2::new(x.size, y.size);
            Vec2::new(x.offset, y.offset)
        }
        Constraint::SetYWithX { x, y, aspect } => {
            let x = x.solve(solve_layout.anchors.x(), parent_size.x, min_size.x);
            let aspect = aspect.unwrap_or_else(|| {
                c_size
                    .map(|cs| cs.size.width / cs.size.height)
                    .unwrap_or(1.)
            });
            let y = y.solve(1. / aspect, x.size, parent_size.y, solve_layout.anchors.y());

            *target_size = Vec2::new(x.size, y.size);
            Vec2::new(x.offset, y.offset)
        }
        Constraint::MaxAspect(aspect) => {
            let aspect = aspect.unwrap_or_else(|| {
                c_size
                    .map(|cs| cs.size.width / cs.size.height)
                    .unwrap_or(1.)
            });
            let x_from_y =
                (solve_layout.anchors.y().1 - solve_layout.anchors.y().0) * parent_size.y * aspect;
            let y_from_x =
                (solve_layout.anchors.x().1 - solve_layout.anchors.x().0) * parent_size.x / aspect;

            *target_size = if x_from_y >= parent_size.x {
                Vec2::new(parent_size.x, y_from_x)
            } else {
                Vec2::new(x_from_y, parent_size.y)
            };
            Vec2::zero()
        }
    };

    if solve_layout.child_constraint.is_some() {
        offset += target_transform.translation.truncate();
    };

    offset += Vec2::new(
        parent_padding.bottom - parent_padding.top,
        parent_padding.left - parent_padding.right,
    ) / 2.;

    target_transform.translation = offset.extend(0.);

    if let Some(children) = children {
        let ts = *target_size;
        if let Some(spread_constraint) = &solve_layout.children_spread {
            let child_nodes = children.iter().map(|c| {
                (
                    nodes
                        .get_component::<AnchorLayout>(*c)
                        .unwrap()
                        .child_constraint
                        .as_ref()
                        .unwrap(),
                    {
                        let size = nodes.get_component::<MinSize>(*c).unwrap().size;
                        match spread_constraint.direction {
                            Direction::Left | Direction::Right => size.x,
                            Direction::Up | Direction::Down => size.y,
                        }
                    },
                    c,
                )
            });

            let ts = ts
                - Vec2::new(
                    solve_layout.padding.left + solve_layout.padding.right,
                    solve_layout.padding.bottom + solve_layout.padding.top,
                );

            let mut free_length = match spread_constraint.direction {
                Direction::Left | Direction::Right => ts.x,
                Direction::Up | Direction::Down => ts.y,
            } - (children.iter().count() - 1) as f32
                * spread_constraint.margin;

            let mut undef = vec![];
            let mut undef_weight_sum = 0.;

            let mut locked = BTreeMap::<usize, (&Entity, f32)>::new();

            for (i, c) in child_nodes.enumerate() {
                undef_weight_sum += c.0.weight;
                undef.push((
                    i,
                    c.0.weight,
                    c.0.min_size.unwrap_or_else(|| c.1),
                    c.0.max_size.unwrap_or_else(|| c.1),
                    c.2,
                ));
            }

            loop {
                let mut dirty = BTreeMap::<_, Vec<_>>::new();
                let length_per_weight = free_length / undef_weight_sum;
                let mut delta = 0.;

                {
                    let mut i = 0;
                    while i != undef.len() {
                        let (_, weight, min, max, _) = undef[i];
                        let len = length_per_weight * weight;
                        if !(min..max).contains(&len) {
                            let clamped = len.clamp(min, max);
                            delta += clamped - len;
                            let entry = dirty.entry(FloatOrd(clamped - len));
                            entry.or_default().push((undef.swap_remove(i), clamped));
                        } else {
                            i += 1;
                        }
                    }
                }

                if dirty.is_empty() {
                    for (i, weight, .., entity) in undef.iter() {
                        let len = length_per_weight * weight;
                        locked.insert(*i, (entity, len));
                    }
                    break;
                } else {
                    let key = if delta > 0. {
                        *dirty.keys().next_back().unwrap()
                    } else {
                        *dirty.keys().next().unwrap()
                    };
                    let ((i, weight, .., entity), clamped) = dirty.entry(key).or_default().pop().unwrap();
                    locked.insert(i, (entity, clamped));
                    free_length -= clamped;
                    undef_weight_sum -= weight;
                    for (v, _) in dirty.into_iter().map(|v| v.1).flatten() {
                        undef.push(v);
                    }
                }
            }

            let (calc_pos, calc_size): (fn(f32, f32, Vec2) -> Vec2, fn(f32, Vec2) -> Vec2) =
                match spread_constraint.direction {
                    Direction::Up => (
                        |size, offset, ts| Vec2::new(0., offset + size / 2. - ts.y / 2.),
                        |size, ts| Vec2::new(ts.x, size),
                    ),
                    Direction::Down => (
                        |size, offset, ts| Vec2::new(0., ts.y / 2. - offset - size / 2.),
                        |size, ts| Vec2::new(ts.x, size),
                    ),
                    Direction::Left => (
                        |size, offset, ts| Vec2::new(ts.x / 2. - offset - size / 2., 0.),
                        |size, ts| Vec2::new(size, ts.y),
                    ),
                    Direction::Right => (
                        |size, offset, ts| Vec2::new(offset + size / 2. - ts.x / 2., 0.),
                        |size, ts| Vec2::new(size, ts.y),
                    ),
                };

            let mut offset = 0.;
            let mut cache = vec![];

            let padding_offset = Vec3::new(
                solve_layout.padding.bottom - solve_layout.padding.top,
                solve_layout.padding.left - solve_layout.padding.right,
                0.,
            ) / 2.;

            for &(&entity, size) in locked.values() {
                let mut transform = mutables.get_component_mut::<Transform>(entity).unwrap();
                transform.translation = calc_pos(size, offset, ts).extend(0.) + padding_offset;
                offset += size + spread_constraint.margin;
                let size = calc_size(size, ts);
                cache.push(size);
                solve(entity, size, Rect::all(0.), false, nodes, mutables);
            }
            let mut target_cache = mutables
                .get_component_mut::<LayoutCache>(solve_entity)
                .unwrap();
            target_cache.children_sizes = Some(cache);
        } else {
            for child in children.iter() {
                solve(*child, ts, solve_layout.padding, false, nodes, mutables);
            }
        }
    }
}

impl AxisConstraint {
    fn solve(
        self,
        anchors: (f32, f32),
        true_space: f32,
        // Only used if `self` is `FromContentSize`
        content_size: f32,
    ) -> AxisConstraintSolve {
        let space = (anchors.1 - anchors.0) * true_space;

        let (p1, s) = match self {
            AxisConstraint::DoubleMargin(p1, p2) => (p1, space - p1 - p2),
            AxisConstraint::DirectMarginAndSize(p1, s) => (p1, s),
            AxisConstraint::ReverseMarginAndSize(p2, s) => (space - p2 - s, s),
            AxisConstraint::Centered(s) => ((space - s) / 2., s),
            AxisConstraint::FromContentSize(alignment) => match alignment {
                Alignment::DirectMargin(v) => (v, content_size),
                Alignment::ReverseMargin(v) => {
                    return AxisConstraintSolve {
                        offset: true_space * (anchors.1 - 0.5) - v - content_size / 2.,
                        size: content_size,
                    }
                }
                Alignment::Offset(offset) => {
                    let int = AxisConstraint::Centered(content_size).solve(
                        anchors,
                        true_space,
                        content_size,
                    );
                    return AxisConstraintSolve {
                        offset: int.offset + offset,
                        size: content_size,
                    };
                }
            },
        };
        let offset = true_space * (anchors.0 - 0.5) + p1 + s / 2.;
        AxisConstraintSolve { offset, size: s }
    }
}

struct AxisConstraintSolve {
    offset: f32,
    size: f32,
}

impl Alignment {
    fn solve(
        &self,
        aspect: f32,
        opposite_size: f32,
        space: f32,
        anchors: (f32, f32),
    ) -> AxisConstraintSolve {
        match self {
            Alignment::DirectMargin(m) => {
                AxisConstraint::DirectMarginAndSize(*m, opposite_size * aspect)
                    .solve(anchors, space, 0.)
            }
            Alignment::ReverseMargin(m) => {
                AxisConstraint::ReverseMarginAndSize(*m, opposite_size * aspect)
                    .solve(anchors, space, 0.)
            }
            Alignment::Offset(o) => AxisConstraintSolve {
                offset: *o,
                size: opposite_size * aspect,
            },
        }
    }
}
